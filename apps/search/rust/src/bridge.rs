use crate::models;

#[cxx::bridge]
pub mod ffi {
    #[derive(Debug)]
    struct Label {
        id: String,
        score: f64,
    }

    #[derive(Debug)]
    struct Engagement {
        comments: i64,
        views: i64,
    }

    #[derive(Debug)]
    struct Candidate {
        id: i64,
        title: String,
        description: String,
        labels: Vec<Label>,
        created_at_unix_seconds: i64,
        likes: i64,
        engagement: Engagement,
    }

    #[derive(Debug)]
    struct SearchInput {
        query: String,
        labels: Vec<Label>,

        now_unix_seconds: i64,
    }

    #[derive(Debug)]
    struct RankedResult {
        id: i64,
        score: f64,
    }

    unsafe extern "C++" {
        include!("scoring.h");

        // transfers ownership of input and candidates to C++
        // C++ returns ownership of the ranked results to Rust
        fn rank_candidates(
            input: SearchInput,
            candidates: Vec<Candidate>,
        ) -> Result<Vec<RankedResult>>;
    }
}

// convert rust models into a shared bridge type
// owned strings and vectors are moved, rather than cloned
impl From<models::Label> for ffi::Label {
    fn from (label: models::Label) -> Self {
        Self {
            id: label.id,
            score: label.score,
        }
    }
}

impl From<models::Engagement> for ffi::Engagement {
    fn from (engagement: models::Engagement) -> Self {
        Self {
            comments: engagement.comments,
            views: engagement.views,
        }
    }
}

impl From<models::Candidate> for ffi::Candidate {
    fn from (candidate: models::Candidate) -> Self {
        Self {
            id: candidate.id,
            title: candidate.title,
            description: candidate.description,
            labels: candidate.labels.into_iter().map(Into::into).collect(),
            created_at_unix_seconds: candidate.created_at.timestamp(),
            likes: candidate.likes,
            engagement: candidate.engagement.into(),
        }
    }
}

// after struct returned 
// convert returnned C++ results into Rust's JSON response model
impl From<ffi::RankedResult> for models::RankedResult {
    fn from (result: ffi::RankedResult) -> Self {
        Self {
            id: result.id,
            score: result.score,
        }
    }
}

pub async fn rank(
    input: models::CandidateRequest,
    candidates: Vec<models::Candidate>,
    permit: tokio::sync::OwnedSemaphorePermit,
) -> Result<Vec<models::RankedResult>, String> {
    let input = ffi::SearchInput {
        query: input.query,
        labels: input.labels.into_iter().map(Into::into).collect(),
        now_unix_seconds: chrono::Utc::now().timestamp(),
    };

    let candidates: Vec<ffi::Candidate> =
        candidates.into_iter().map(Into::into).collect();

    tokio::task::spawn_blocking(move || {
        let _permit = permit;

        ffi::rank_candidates(input, candidates)
            .map(|results| results.into_iter().map(Into::into).collect())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}