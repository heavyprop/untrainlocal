"""Fetch full, unranked candidate data from Rust for a future ranking stage."""

import json
import math
from datetime import datetime
from urllib.request import Request, urlopen

from django.conf import settings


def _validate_response(result):
    if not isinstance(result, dict):
        raise ValueError("Expected a candidate response object")
    terms = result.get("query_terms")
    candidates = result.get("candidates")
    if not isinstance(terms, list) or not all(isinstance(t, str) for t in terms):
        raise ValueError("Invalid query terms")
    if not isinstance(candidates, list):
        raise ValueError("Expected unranked candidates, not ranked results")
    for candidate in candidates:
        if not isinstance(candidate, dict) or type(candidate.get("id")) is not int:
            raise ValueError("Invalid candidate ID")
        if not all(
            isinstance(candidate.get(k), str)
            for k in ("title", "description", "created_at")
        ):
            raise ValueError("Invalid candidate text or timestamp")
        if datetime.fromisoformat(candidate["created_at"]).utcoffset() is None:
            raise ValueError("Candidate timestamp must include a timezone")
        engagement = candidate.get("engagement")
        if not isinstance(engagement, dict):
            raise ValueError("Missing engagement counts")
        for value in (
            candidate.get("likes"),
            engagement.get("comments"),
            engagement.get("views"),
        ):
            if type(value) is not int or value < 0:
                raise ValueError("Invalid engagement count")
        labels = candidate.get("labels")
        if not isinstance(labels, list):
            raise ValueError("Invalid candidate labels")
        for label in labels:
            if not isinstance(label, dict):
                raise ValueError("Invalid candidate label")
            score = label.get("score")
            if (
                not isinstance(label.get("id"), str)
                or not label["id"].strip()
                or type(score) not in (int, float)
                or not math.isfinite(score)
                or not 0 <= score <= 1
            ):
                raise ValueError("Invalid candidate label")
    return result


def fetch_candidates(query, labels, *, url=None):
    """Return query_terms and candidate dictionaries; raise on service/schema errors.

    This does not rank candidates or silently fall back to another search backend.
    """
    endpoint = url or getattr(
        settings, "RUST_CANDIDATES_URL", "http://127.0.0.1:8081/candidates"
    )
    request = Request(
        endpoint,
        data=json.dumps({"query": query, "labels": labels}, allow_nan=False).encode(),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urlopen(request, timeout=20) as response:
        return _validate_response(json.load(response))
