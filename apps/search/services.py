"""Classify, call Rust/C++ search, then hydrate or fall back to keywords."""

from apps.recommendations.classification import get_top_labels

from .client import search_with_service
from .fallback import keyword_search


def search_threads(query):
    if not query:
        return [], [], []

    query_labels = get_top_labels(query, None)
    result = search_with_service(query, query_labels)

    if result is False:
        query_terms, matches = keyword_search(query)
    else:
        query_terms = result["query_terms"]
        matches = result["results"]

    return query_terms, query_labels, matches 
