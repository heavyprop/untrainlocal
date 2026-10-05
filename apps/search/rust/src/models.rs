use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Label {
    pub id: String,
    pub score: f64,
}

impl Label {
    pub fn is_valid(&self) -> bool {
        !self.id.trim().is_empty()
            && self.id.len() <= 128
            && self.score.is_finite()
            && (0.0..=1.0).contains(&self.score)
    }
}

// Incoming HTTP request.
#[derive(Debug, Deserialize)]
pub struct CandidateRequest {
    pub query: String,

    #[serde(default)]
    pub labels: Vec<Label>,
}

#[derive(Debug, Default, Serialize)]
pub struct Engagement {
    pub comments: i64,
    pub views: i64,
}

// Candidate fetched from PostgreSQL.
#[derive(Debug, Serialize)]
pub struct Candidate {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub labels: Vec<Label>,
    pub created_at: DateTime<Utc>,
    pub likes: i64,
    pub engagement: Engagement,
}

// Final HTTP response once scoring is connected.
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub query_terms: Vec<String>,
    pub results: Vec<RankedResult>,
}

#[derive(Debug, Serialize)]
pub struct RankedResult {
    pub id: i64,
    pub score: f64,
}
