"""Candidate-client contract tests; no database or running service required."""

import io
import json
import unittest
from unittest.mock import patch
from urllib.error import URLError

from .candidates import fetch_candidates


class CandidateClientTests(unittest.TestCase):
    def test_returns_full_candidate_data_and_sends_query_labels(self):
        candidate = {
            "id": 7,
            "title": "Rust",
            "description": "Learning Rust",
            "labels": [{"id": "code", "score": 0.8}],
            "created_at": "2026-01-01T00:00:00Z",
            "likes": 2,
            "engagement": {"comments": 3, "views": 4},
        }
        body = {"query_terms": ["rust"], "candidates": [candidate]}
        with patch(
            "apps.search.candidates.urlopen",
            return_value=io.BytesIO(json.dumps(body).encode()),
        ) as call:
            result = fetch_candidates(
                "rust", [], url="http://localhost:8081/candidates"
            )
        self.assertEqual(result, body)
        request = call.call_args.args[0]
        self.assertEqual(json.loads(request.data), {"query": "rust", "labels": []})
        self.assertEqual(request.method, "POST")
        self.assertNotIn("score", result["candidates"][0])

    def test_rejects_old_ranked_response(self):
        body = {"query_terms": ["rust"], "results": [{"id": 7, "score": 1.0}]}
        with patch(
            "apps.search.candidates.urlopen",
            return_value=io.BytesIO(json.dumps(body).encode()),
        ):
            with self.assertRaises(ValueError):
                fetch_candidates("rust", [], url="http://localhost:8081/candidates")

    def test_service_errors_are_not_hidden_by_fallback(self):
        with patch("apps.search.candidates.urlopen", side_effect=URLError("offline")):
            with self.assertRaises(URLError):
                fetch_candidates("rust", [], url="http://localhost:8081/candidates")
