from django.http import JsonResponse
from django.shortcuts import redirect, render
from django.template.loader import render_to_string
from django.views.decorators.cache import never_cache
from django.views.decorators.http import require_GET

from common.security.rate_limits import limit_requests

from .pagination import get_ranking, load_page, save_ranking
from .services import search_threads

@never_cache
@require_GET
@limit_requests(
    rate="8/m",
    group="search",
    method="GET",
    query_param="q",
)
def search(request):
    if not request.user.is_authenticated:
        return redirect("boarding")

    query = request.GET.get("q", "").strip()

    token = None
    page = {"posts": [], "next_offset": None}

    if query:
        _, _, matches = search_threads(query)
        page = load_page(request.user, matches)

        if page["next_offset"] is not None:
            token = save_ranking(request.user.pk, matches)

    return render(
        request,
        "search/results.html",
        {
            "query": query,
            "search_results": page["posts"],
            "search_token": token,
            "next_offset": page["next_offset"],
            "has_more": page["next_offset"] is not None,
        },
    )

@never_cache
@require_GET
def load_more(request, token):
    if not request.user.is_authenticated:
        return JsonResponse(
            {"error": "Please sign in again."},
            status = 401,
        )

    # including user id preventing reading another users ranking
    matches = get_ranking(request.user.pk, token)

    if matches is None:
        return JsonResponse(
            {"error": "This search has expired. Search again to continue."},
            status=410,
        )

    try:
        offset = int(request.GET.get("offset", "0"))
    except ValueError:
        return JsonResponse({"error": "Invalid offset"}, status=400)

    if offset < 0 or offset > len(matches):
        return JsonResponse({"error": "Invalid offset"}, status=400)

    page = load_page(request.user, matches, offset)

    html = render_to_string(
        "search/partials/result_batch.html",
        {"search_results": page["posts"]},
        request=request,
    )

    return JsonResponse(
        {
            "html": html,
            "count": len(page["posts"]),
            "next_offset": page["next_offset"],
            "has_more": page["next_offset"] is not None,
        }
    )
