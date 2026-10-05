#include "scoring.h"
#include "rust/src/bridge.rs.h"

// from RUST

// SearchInput
// |- query
// |- labels -> multiple label object
// |- now_unix_seconds

// Candidate
// |- id, title, description
// |- labels -> multiple label objects
// |- created_at_unix_seconds, likes
// |- engagement -> one Engagement object
//                   |
//                   |- comments
//                   |- views

// RankedResult
// |- id
// |- score

#include "metrics.h"
#include "text.h"

#include <algorithm>
#include <cstdint>
#include <string_view>
#include <vector>

namespace {

constexpr double TITLE_WEIGHT = 0.7;
constexpr double DESCRIPTION_WEIGHT = 0.3;

constexpr double TEXT_WEIGHT = 0.65;
constexpr double TAG_WEIGHT = 0.35;
constexpr double AGREEMENT_BONUS = 0.3;

constexpr double LIKE_WEIGHT = 0.55;
constexpr double COMMENT_WEIGHT = 0.30;
constexpr double VIEW_WEIGHT = 0.15;

constexpr double FINAL_RELEVANCE_WEIGHT = 0.7;
constexpr double FINAL_RECENCY_WEIGHT = 0.3;

constexpr double MIN_TAG_MATCH = 0.1;
constexpr double MIN_POPULARITY = 0.3;

// Borrow existing UTF8 bytes without copying the string
std::string_view as_view(const rust::String& text) {
    return {text.data(), text.size()};
}

// temporary calcualtions
struct ScoredCandidate {
    std::int64_t id;
    double relevance;
    double recency;
    double engagement;
    bool passes_gate;
};

} //namespace

rust::Vec<RankedResult> rank_candidates(SearchInput input, rust::Vec<Candidate> candidates) {
    if (candidates.empty()) { return {}; }

    // prepare query words and labels once

    const auto query_terms = search_text::tokenize(as_view(input.query));

    search_metrics::LabelScores query_labels;
    query_labels.reserve(input.labels.size());

    for (const auto& label : input.labels) {
        query_labels.insert_or_assign(as_view(label.id), label.score);
    }

    // 2 find max counts for engagement normalization
    std::int64_t max_likes{ 0 };
    std::int64_t max_comments{ 0 };
    std::int64_t max_views{ 0 };

    for (const auto& candidate : candidates) {
        max_likes = std::max(max_likes, candidate.likes);
        max_comments = std::max(max_comments, candidate.engagement.comments);
        max_views = std::max(max_views, candidate.engagement.views);
    }

    // store calculated values not copies of candidates
    std::vector<ScoredCandidate> scored;
    scored.reserve(candidates.size());

    std::vector<double> engagements;
    engagements.reserve(candidates.size());

    // 3. calcualte each candidate component scores
    for (const auto& candidate : candidates) {

        const double text_score = TITLE_WEIGHT * search_text::match_score(query_terms, as_view(candidate.title)) 
            + DESCRIPTION_WEIGHT * search_text::match_score(query_terms, as_view(candidate.description));

        const double tag_score = search_metrics::tag_match_score(query_labels, candidate.labels);

        const double relevance = (TEXT_WEIGHT * text_score + TAG_WEIGHT * tag_score) 
            * (1.0 + AGREEMENT_BONUS * text_score * tag_score);

        const double engagement = 
            LIKE_WEIGHT * search_metrics::normalize_count(candidate.likes, max_likes)
            + COMMENT_WEIGHT * search_metrics::normalize_count(candidate.engagement.comments, max_comments)
            + VIEW_WEIGHT * search_metrics::normalize_count(candidate.engagement.views, max_views);

        const double recency = search_metrics::recency_score(candidate.created_at_unix_seconds, input.now_unix_seconds);

        const bool passes_gate = tag_score >= MIN_TAG_MATCH || text_score > 0.0;

        scored.push_back(ScoredCandidate{
            candidate.id,
            relevance,
            recency,
            engagement,
            passes_gate
        });

        engagements.push_back(engagement);
    }

    // 4. use all retrieved candidates for the engagement baseline
    const double baseline = search_metrics::median(engagements);

    // 5. filter candidates and apply the final formula
    rust::Vec<RankedResult> results;
    results.reserve(scored.size());

    for (const auto& candidate : scored) {
        if (!candidate.passes_gate) {
            continue;
        }

        const double popularity = std::max(1.0 + candidate.engagement - baseline, MIN_POPULARITY);

        const double final_score = (FINAL_RELEVANCE_WEIGHT * candidate.relevance + FINAL_RECENCY_WEIGHT * candidate.recency) * popularity;

        results.push_back(RankedResult{
            candidate.id,
            final_score
        });
    }

    std::sort(
        results.begin(),
        results.end(),
        [](const RankedResult& a, const RankedResult& b) {
            if (a.score != b.score) {
                return a.score > b.score;
            }

            return a.id < b.id;
        }
    );

    return results;

}
