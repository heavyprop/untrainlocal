#include "metrics.h"
#include "rust/src/bridge.rs.h"

#include <algorithm>
#include <cmath>

namespace search_metrics {

namespace {

constexpr double SECONDS_PER_DAY = 86'400.0;
constexpr double HALF_LIFE_DAYS = 14.0;

}

double tag_match_score(const LabelScores& query_labels, const rust::Vec<Label>& post_labels) {
    LabelScores post_scores;
    post_scores.reserve(post_labels.size());


    for (const auto& label : post_labels) {
        const std::string_view id(label.id.data(), label.id.size());
        post_scores.insert_or_assign(id, label.score);
    }

    double score{ 0.0 };

    for (const auto& [id, confidence] : query_labels) {
        const auto match = post_scores.find(id);

        if (match != post_scores.end()) {
            score += confidence * match->second;
        }

    }

    return std::min(score, 1.0);
}

double normalize_count(std::int64_t count, std::int64_t maximum) {
    if (maximum <= 0) { return 0.0; }

    const double nonnegative_count = static_cast<double>(std::max<std::int64_t>(count, 0));

    return std::log1p(nonnegative_count) / std::log1p(static_cast<double>(maximum));
}

double recency_score(std::int64_t created_at_unix_seconds, std::int64_t now_unix_seconds) {
    const double age_seconds = std::max(static_cast<double>(now_unix_seconds) - static_cast<double>(created_at_unix_seconds), 0.0);

    const double age_days = age_seconds / SECONDS_PER_DAY;

    return std::pow(0.5, age_days / HALF_LIFE_DAYS);
}

double median(std::vector<double>& values) {
    if (values.empty()) { return 0.0; }

    std::sort(values.begin(), values.end());

    const std::size_t middle = values.size() / 2;

    if (values.size() % 2 == 0) {
        return (values[middle - 1] + values[middle]) / 2.0;
    } 

    return values[middle];
}

}
