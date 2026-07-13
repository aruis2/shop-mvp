use async_trait::async_trait;
use sqlx::PgPool;

use crate::models::*;
use crate::{ArticleFilter, ArticleWithScore, KnowledgeError, KnowledgeRepo, KnowledgeStats};

/// Implementare PostgreSQL a `KnowledgeRepo`
pub struct PgKnowledgeRepo {
    pool: PgPool,
}

impl PgKnowledgeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl KnowledgeRepo for PgKnowledgeRepo {
    async fn migrate(&self) -> Result<(), KnowledgeError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS articles (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                title TEXT NOT NULL,
                slug TEXT UNIQUE NOT NULL,
                summary TEXT,
                content TEXT NOT NULL,
                category_path TEXT[],
                tags TEXT[],
                difficulty TEXT,
                related_concepts TEXT[],
                reading_time_minutes INTEGER,
                published_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            );
            CREATE TABLE IF NOT EXISTS article_embeddings (
                id UUID PRIMARY KEY REFERENCES articles(id) ON DELETE CASCADE,
                embedding vector(768),
                model TEXT NOT NULL DEFAULT 'nomic-embed-text',
                updated_at TIMESTAMPTZ DEFAULT NOW()
            );
            "#
        )
        .execute(&self.pool)
        .await?;

        // Indexuri (IF NOT EXISTS pentru GIN/IVFFlat nu e suportat, folosim try)
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_slug ON articles(slug)").execute(&self.pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_tags ON articles USING GIN(tags)").execute(&self.pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_category ON articles USING GIN(category_path)").execute(&self.pool).await;
        let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_fts ON articles USING GIN(to_tsvector('romanian', title || ' ' || COALESCE(summary, '') || ' ' || content))").execute(&self.pool).await;

        Ok(())
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<Article>, KnowledgeError> {
        let article = sqlx::query_as::<_, Article>(
            r#"
            SELECT id, title, slug, summary, content, category_path, tags,
                   difficulty, related_concepts, reading_time_minutes,
                   published_at, updated_at
            FROM articles WHERE slug = $1
            "#
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(article)
    }

    async fn list(&self, filter: &ArticleFilter) -> Result<Vec<Article>, KnowledgeError> {
        let order = match filter.sort.as_deref() {
            Some("newest") => "published_at DESC NULLS LAST",
            Some("oldest") => "published_at ASC NULLS LAST",
            Some("longest") => "reading_time_minutes DESC NULLS LAST",
            Some("shortest") => "reading_time_minutes ASC NULLS LAST",
            _ => "category_path[1], published_at DESC",
        };

        use sqlx::QueryBuilder;
        let mut qb = QueryBuilder::new(
            "SELECT id, title, slug, summary, content, category_path, tags,
                    difficulty, related_concepts, reading_time_minutes,
                    published_at, updated_at
             FROM articles"
        );

        if let Some(ref diff) = filter.difficulty {
            qb.push(" WHERE difficulty = ");
            qb.push_bind(diff);
        }

        qb.push(" ORDER BY ");
        qb.push(order);

        let articles = qb.build_query_as::<Article>()
            .fetch_all(&self.pool)
            .await?;
        Ok(articles)
    }

    async fn search_fts(&self, query: &str) -> Result<Vec<Article>, KnowledgeError> {
        let search_pattern = format!("%{}%", query);
        let articles = sqlx::query_as::<_, Article>(
            r#"
            SELECT id, title, slug, summary, content, category_path, tags,
                   difficulty, related_concepts, reading_time_minutes,
                   published_at, updated_at
            FROM articles
            WHERE to_tsvector('romanian', title || ' ' || COALESCE(summary, '') || ' ' || content)
                  @@ plainto_tsquery('romanian', $1)
               OR title ILIKE $2
               OR COALESCE(summary, '') ILIKE $2
            ORDER BY ts_rank(
                to_tsvector('romanian', title || ' ' || COALESCE(summary, '') || ' ' || content),
                plainto_tsquery('romanian', $1)
            ) DESC
            LIMIT 50
            "#
        )
        .bind(query)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await?;
        Ok(articles)
    }

    async fn search_semantic(&self, embedding: &[f32], limit: i64) -> Result<Vec<ArticleWithScore>, KnowledgeError> {
        let emb_str: Vec<String> = embedding.iter().map(|v| v.to_string()).collect();
        let emb_array = emb_str.join(",");

        use sqlx::QueryBuilder;
        let mut qb = QueryBuilder::new(
            "SELECT a.id, a.title, a.slug, a.summary, a.content,
                    a.category_path, a.tags, a.difficulty,
                    a.related_concepts, a.reading_time_minutes,
                    a.published_at, a.updated_at,
                    1 - (ae.embedding <=> ARRAY["
        );
        qb.push(&emb_array);
        qb.push("]::vector) AS similarity
             FROM articles a
             JOIN article_embeddings ae ON a.id = ae.id
             ORDER BY similarity DESC
             LIMIT ");
        qb.push(limit);

        #[derive(sqlx::FromRow)]
        struct ArticleRow {
            id: uuid::Uuid, title: String, slug: String, summary: Option<String>,
            content: String, category_path: Option<Vec<String>>, tags: Option<Vec<String>>,
            difficulty: Option<String>, related_concepts: Option<Vec<String>>,
            reading_time_minutes: Option<i32>, published_at: Option<chrono::DateTime<chrono::Utc>>,
            updated_at: Option<chrono::DateTime<chrono::Utc>>,
            similarity: Option<f64>,
        }

        let rows = qb.build_query_as::<ArticleRow>()
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| ArticleWithScore {
            article: Article {
                id: r.id, title: r.title, slug: r.slug,
                summary: r.summary, content: r.content,
                category_path: r.category_path, tags: r.tags,
                difficulty: r.difficulty, related_concepts: r.related_concepts,
                reading_time_minutes: r.reading_time_minutes,
                published_at: r.published_at, updated_at: r.updated_at,
            },
            similarity: r.similarity.unwrap_or(0.0),
        }).collect())
    }

    async fn stats(&self) -> Result<KnowledgeStats, KnowledgeError> {
        let total: i64 =
            sqlx::query_scalar::<_, Option<i64>>("SELECT COUNT(*) FROM articles")
                .fetch_one(&self.pool).await?.unwrap_or(0);

        let total_minutes: i64 =
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COALESCE(SUM(reading_time_minutes), 0) FROM articles"
            )
            .fetch_one(&self.pool).await?.unwrap_or(0);

        let category_count: i64 =
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COUNT(DISTINCT category_path[1]) FROM articles WHERE category_path IS NOT NULL"
            )
            .fetch_one(&self.pool).await?.unwrap_or(0);

        let difficulty_breakdown: Vec<DifficultyCount> = sqlx::query_as(
            "SELECT difficulty, COUNT(*) as count FROM articles GROUP BY difficulty ORDER BY difficulty"
        )
        .fetch_all(&self.pool).await?;

        Ok(KnowledgeStats {
            total_articles: total,
            total_reading_minutes: total_minutes,
            category_count,
            difficulty_breakdown,
        })
    }

    async fn get_all(&self) -> Result<Vec<Article>, KnowledgeError> {
        let articles = sqlx::query_as::<_, Article>(
            r#"
            SELECT id, title, slug, summary, content, category_path, tags,
                   difficulty, related_concepts, reading_time_minutes,
                   published_at, updated_at
            FROM articles
            ORDER BY category_path[1], published_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(articles)
    }
}
