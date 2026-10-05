"""Ranked search service contract; no database or ML model required."""

import io
import json
import unittest
from types import SimpleNamespace
from unittest.mock import patch

from .client import search_with_service


class SearchClientTests(unittest.TestCase):
    def test_uses_configured_service_and_preserves_ranked_results(self):
        response = {
            "query_terms": ["rust"],
            "results": [{"id": 7, "score": 1.21}, {"id": 2, "score": 0.5}],
        }
        labels = [{"id": "code", "score": 0.8}]
        with (
            patch(
                "apps.search.client.settings",
                SimpleNamespace(
                    SEARCH_URL="http://search:8080/search",
                    SEARCH_SERVICE_TOKEN="unit-test-token",
                ),
            ),
            patch(
                "apps.search.client.urlopen",
                return_value=io.BytesIO(json.dumps(response).encode()),
            ) as call,
        ):
            self.assertEqual(search_with_service("rust", labels), response)

        request = call.call_args.args[0]
        self.assertEqual(request.full_url, "http://search:8080/search")
        self.assertEqual(request.method, "POST")
        self.assertEqual(request.get_header("Authorization"), "Bearer unit-test-token")
        self.assertEqual(json.loads(request.data), {"query": "rust", "labels": labels})

    def test_candidate_only_response_triggers_fallback(self):
        with (
            patch(
                "apps.search.client.settings",
                SimpleNamespace(
                    SEARCH_URL="http://search:8080/search",
                    SEARCH_SERVICE_TOKEN="unit-test-token",
                ),
            ),
            patch(
                "apps.search.client.urlopen",
                return_value=io.BytesIO(b'{"query_terms":[],"candidates":[]}'),
            ),
            patch("apps.search.client.logger.exception"),
        ):
            self.assertIs(search_with_service("rust", []), False)
