#pragma once

#include "rust/cxx.h"

#include <cstdint>
#include <string_view>
#include <unordered_map>
#include <vector>

// defined by the generated bridge header
struct Label;

namespace search_metrics {

// borrows label IDs; their original strings must remain alive
using LabelScores = std::unordered_map<std::string_view, double>;

// compare query and post confidence scores for matching label ids
// cap the combined score at 1.0
double tag_match_score(const LabelScores& query_labels, const rust::Vec<Label>& post_labels);

// logarithmic scaling relative to the highest candidate count
//return 0.0 when maximum is zero or negative
double normalize_count(std::int64_t count, std::int64_t maximum);

// freshness halves every 14 days
// both time stamps are Unix seconds; future dates score 1.0
double recency_score(std::int64_t created_at_unix_seconds, std::int64_t now_unix_seconds);

// sorts values in place and returns their median
// return 0.0 for empty vector
double median(std::vector<double>& values);
} // namespace search_metrics
