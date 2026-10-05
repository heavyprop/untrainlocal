#pragma once

#include "rust/cxx.h"

struct SearchInput;
struct Candidate;
struct RankedResult;

rust::Vec<RankedResult> rank_candidates(
    SearchInput input,
    rust::Vec<Candidate> candidates
);
