use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// O categorie din baza de date
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<i32>,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

/// Un element din breadcrumb (mersul înapoi spre rădăcină)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CategoryBreadcrumb {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub level: i32,
}

/// Un nod din arborele de categorii (cu copii incluși)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryView {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub icon: String,
    pub description: String,
    pub children: Vec<CategoryView>,
}
