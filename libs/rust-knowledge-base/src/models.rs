use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Un articol din knowledge base
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Article {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub summary: Option<String>,
    pub content: String,
    pub category_path: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub difficulty: Option<String>,
    pub related_concepts: Option<Vec<String>>,
    pub reading_time_minutes: Option<i32>,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Articol cu scor de similaritate (semantic search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleWithScore {
    pub article: Article,
    pub similarity: f64,
}

/// Filtre pentru listarea articolelor
#[derive(Debug, Clone, Default)]
pub struct ArticleFilter {
    pub difficulty: Option<String>,
    pub sort: Option<String>,       // "newest" | "oldest" | "longest" | "shortest"
}

/// Statistici knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeStats {
    pub total_articles: i64,
    pub total_reading_minutes: i64,
    pub category_count: i64,
    pub difficulty_breakdown: Vec<DifficultyCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DifficultyCount {
    pub difficulty: Option<String>,
    pub count: Option<i64>,
}

/// Document RAG gata de embed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagDocument {
    pub id: String,
    pub doc_type: String,
    pub content: String,
    pub title: String,
    pub metadata: serde_json::Value,
    pub url: String,
}
