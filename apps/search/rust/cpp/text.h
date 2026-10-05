#pragma once

#include <string>
#include <string_view>
#include <vector>

namespace search_text {

std::vector<std::string> tokenize(std::string_view text);

double match_score(
    const std::vector<std::string>& query_terms,
    std::string_view text
);

}
