//! # rust-knowledge-base
//!
//! Articole, căutare full-text + semantică, statistici.
//!
//! ## Teste
//! ```bash
//! cargo test -p rust-knowledge-base
//! ```

mod models;
mod pg;

pub use models::{
    Article, ArticleWithScore, ArticleFilter, KnowledgeStats,
    DifficultyCount, RagDocument,
};
pub use pg::PgKnowledgeRepo;

use async_trait::async_trait;
use uuid::Uuid;
use thiserror::Error;

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, Error)]
pub enum KnowledgeError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Article not found: {0}")]
    NotFound(String),

    #[error("Semantic search unavailable: {0}")]
    SemanticSearchError(String),
}

// ============================================================================
// Trait — KnowledgeRepo
// ============================================================================

#[async_trait]
pub trait KnowledgeRepo: Send + Sync {
    /// Creează tabelele `articles` + `article_embeddings`
    async fn migrate(&self) -> Result<(), KnowledgeError>;

    /// Obține un articol după slug
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Article>, KnowledgeError>;

    /// Listare articole cu filtre
    async fn list(&self, filter: &ArticleFilter) -> Result<Vec<Article>, KnowledgeError>;

    /// Căutare full-text PostgreSQL
    async fn search_fts(&self, query: &str) -> Result<Vec<Article>, KnowledgeError>;

    /// Căutare semantică (pgvector) — primește embedding-ul deja generat
    async fn search_semantic(&self, embedding: &[f32], limit: i64) -> Result<Vec<ArticleWithScore>, KnowledgeError>;

    /// Statistici
    async fn stats(&self) -> Result<KnowledgeStats, KnowledgeError>;

    /// Obține toate articolele (pentru training/RAG export)
    async fn get_all(&self) -> Result<Vec<Article>, KnowledgeError>;

    /// Convertește un articol în document RAG
    fn article_to_rag(&self, article: &Article) -> RagDocument {
        RagDocument {
            id: format!("article:{}", article.slug),
            doc_type: "article".into(),
            content: format!(
                "Titlu: {}\n\nRezumat: {}\n\nConținut: {}\n\nDificultate: {}\n\nConcepte: {}",
                article.title,
                article.summary.as_deref().unwrap_or(""),
                strip_frontmatter(&article.content),
                article.difficulty.as_deref().unwrap_or("general"),
                article.related_concepts.as_ref()
                    .map(|c| c.join(", "))
                    .unwrap_or_default(),
            ),
            title: article.title.clone(),
            metadata: serde_json::json!({
                "slug": article.slug,
                "difficulty": article.difficulty,
                "tags": article.tags,
                "reading_time": article.reading_time_minutes,
            }),
            url: format!("/biblioteca/{}", article.slug),
        }
    }
}

/// Elimină frontmatter-ul YAML (--- ... ---) din conținut
pub fn strip_frontmatter(content: &str) -> &str {
    let s = content.trim();
    if s.starts_with("---") {
        if let Some(end) = s[3..].find("---") {
            return s[3 + end + 3..].trim();
        }
    }
    s
}

// ============================================================================
// Teste — fără DB, doar logică pură
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_frontmatter() {
        let content = "---\ntitle: Test\nslug: test\n---\n\n# Articolul meu\n\nConținut.";
        let result = strip_frontmatter(content);
        assert_eq!(result.trim(), "# Articolul meu\n\nConținut.");
    }

    #[test]
    fn test_strip_frontmatter_no_frontmatter() {
        let content = "# Doar articol\nfără frontmatter.";
        assert_eq!(strip_frontmatter(content), content);
    }

    #[test]
    fn test_article_to_rag_contains_title() {
        let article = Article {
            id: Uuid::nil(),
            title: "Test Article".into(),
            slug: "test-article".into(),
            summary: Some("Un rezumat.".into()),
            content: "---\n---\n\nConținut real.".into(),
            category_path: None, tags: None, difficulty: Some("beginner".into()),
            related_concepts: None, reading_time_minutes: Some(5),
            published_at: None, updated_at: None,
        };
        let rag = KnowledgeRepo::article_to_rag(&MockRepo, &article);
        assert!(rag.content.contains("Test Article"));
        assert!(rag.content.contains("Conținut real."));
        assert_eq!(rag.doc_type, "article");
        assert_eq!(rag.url, "/biblioteca/test-article");
    }

    struct MockRepo;
    #[async_trait]
    impl KnowledgeRepo for MockRepo {
        async fn migrate(&self) -> Result<(), KnowledgeError> { Ok(()) }
        async fn get_by_slug(&self, _: &str) -> Result<Option<Article>, KnowledgeError> { Ok(None) }
        async fn list(&self, _: &ArticleFilter) -> Result<Vec<Article>, KnowledgeError> { Ok(vec![]) }
        async fn search_fts(&self, _: &str) -> Result<Vec<Article>, KnowledgeError> { Ok(vec![]) }
        async fn search_semantic(&self, _: &[f32], _: i64) -> Result<Vec<ArticleWithScore>, KnowledgeError> { Ok(vec![]) }
        async fn stats(&self) -> Result<KnowledgeStats, KnowledgeError> { unimplemented!() }
        async fn get_all(&self) -> Result<Vec<Article>, KnowledgeError> { Ok(vec![]) }
    }

    #[test]
    fn test_article_filter_default() {
        let filter = ArticleFilter::default();
        assert!(filter.difficulty.is_none());
        assert!(filter.sort.is_none());
    }

    #[test]
    fn test_article_filter_with_values() {
        let filter = ArticleFilter {
            difficulty: Some("advanced".into()),
            sort: Some("newest".into()),
        };
        assert_eq!(filter.difficulty.unwrap(), "advanced");
        assert_eq!(filter.sort.unwrap(), "newest");
    }
}


