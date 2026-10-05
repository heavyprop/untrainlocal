use crate::{
    bridge,
    models::{CandidateRequest, SearchResponse},
    repository, text,
};
use sqlx::{Error as DbError, PgPool};
use tokio::sync::OwnedSemaphorePermit;

#[derive(Debug)]
pub enum SearchError {
    InvalidInput(&'static str),
    Database(DbError),
    Scoring(String),
}

impl From<DbError> for SearchError {
    fn from(error: DbError) -> Self {
        Self::Database(error)
    }
}

fn validate(input: &CandidateRequest) -> Result<(), SearchError> {
    if input.query.len() > 512 {
        return Err(SearchError::InvalidInput("Query must be at most 512 bytes"));
    }

    if input.labels.len() > 32 || input.labels.iter().any(|label| !label.is_valid()) {
        return Err(SearchError::InvalidInput(
            "Supply at most 32 labels with nonempty IDs and scores from 0 to 1",
        ));
    }

    Ok(())
}

pub async fn run(
    db: &PgPool,
    input: CandidateRequest,
    candidate_limit: u64,
    permit: OwnedSemaphorePermit,
) -> Result<SearchResponse, SearchError> {
    validate(&input)?;

    let terms = text::tokenize(&input.query);

    if terms.len() > 32 {
        return Err(SearchError::InvalidInput(
            "Query must contain at most 32 unique words",
        ));
    }

    // Rust asynchronously fetches candidates.
    let candidates =
        repository::fetch_candidates(db, &terms, &input.labels, candidate_limit).await?;

    // C++ scores them on a blocking worker.
    let results = bridge::rank(input, candidates, permit)
        .await
        .map_err(SearchError::Scoring)?;

    Ok(SearchResponse {
        query_terms: terms,
        results,
    })
}
