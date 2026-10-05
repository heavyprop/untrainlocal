"""Keyword-only search used when the Rust/C++ service is unavailable.

This intentionally has no ranking formula: matching posts are newest first.
Tokenization stays here so the healthy service path does not repeat that work.
"""

import re

from django.db.models import Q

from apps.discussions.models import Thread

TOKEN_RE = re.compile(r"[a-z0-9+#.]+")
STOP_WORDS = frozenset(
    "a an and are as at be best but by for from has have how i if in is it "
    "of on or should that the this to was what when where which who why with you".split()
)


def keyword_search(query):
    terms = list(
        dict.fromkeys(
            term
            for term in TOKEN_RE.findall(query.lower())
            if len(term) >= 2 and term not in STOP_WORDS
        )
    )
    candidate_filter = Q()
    for term in terms or [query]:
        candidate_filter |= Q(title__icontains=term) | Q(description__icontains=term)

    post_ids = (
        Thread.objects.all()
        .filter(candidate_filter)
        .order_by("-created_at", "-id")
        .values_list("id", flat=True)[:50]
    )

    matches = [{"id": post_id, "score": 0.0} for post_id in post_ids]
    return terms, matches 
