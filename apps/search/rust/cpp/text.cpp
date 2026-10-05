#include "text.h"

#include <algorithm>
#include <cstdint>
#include <limits>
#include <stdexcept>
#include <unordered_set>
#include <utility>

#include <unicode/locid.h>
#include <unicode/stringpiece.h>
#include <unicode/uchar.h>
#include <unicode/unistr.h>
#include <unicode/utf16.h>

namespace search_text {
namespace {

bool is_word_character(UChar32 character) {
    const auto category = u_charType(character);

    return u_hasBinaryProperty(character, UCHAR_ALPHABETIC)
        || category == U_DECIMAL_DIGIT_NUMBER
        || category == U_LETTER_NUMBER
        || category == U_OTHER_NUMBER;
}
} // namespace

std::vector<std::string> tokenize(std::string_view text) {
    if (text.empty()) { return {}; }

    if (text.size() > static_cast<std::size_t>(std::numeric_limits<std::int32_t>::max())) {
        throw std::length_error("Text is too large to tokenize");
    }

    auto lowercase = icu::UnicodeString::fromUTF8(
        icu::StringPiece(
            text.data(),
            static_cast<std::int32_t>(text.size())
        )
    );

    lowercase.toLower(icu::Locale::getRoot());

    if (lowercase.isBogus()) { throw std::runtime_error("Unicode text conversion failed"); }

    std::vector<std::string> tokens;
    icu::UnicodeString word;

    auto finish_word = [&]() {
        if (word.isEmpty()) { return; }

        std::string token;
        word.toUTF8String(token);
        tokens.push_back(std::move(token));
        word.remove();
    };

    for (std::int32_t index = 0; index < lowercase.length();) {
        const UChar32 character = lowercase.char32At(index);
        index += U16_LENGTH(character);

        if (is_word_character(character)) {
            word.append(character);
        } else {
            finish_word();
        }
    }

    finish_word();

    // sort so duplicates are adjacent, then remove them.
    std::sort(tokens.begin(), tokens.end());
    tokens.erase(
        std::unique(tokens.begin(), tokens.end()),
        tokens.end()
    );

    return tokens;
}

double match_score(const std::vector<std::string>& query_terms, std::string_view text) {
    if (query_terms.empty()) {
        return 0.0;
    }

    const auto tokens = tokenize(text);
    const std::unordered_set<std::string_view> words(tokens.begin(), tokens.end());

    std::size_t matches = 0;

    // query_terms must come from tokenize()
    for (const auto& term : query_terms) {
        if (words.find(term) != words.end()) {
            ++matches;
        }
    }

    return static_cast<double>(matches) / static_cast<double>(query_terms.size());
}
}
