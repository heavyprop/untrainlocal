from uuid import uuid4

from django.core.cache import cache

from apps.accounts.models import UserBlock
from apps.discussions.models import Thread

from .selectors import hydrate_ranked_posts

PAGE_SIZE = 10
SEARCH_TTL = 15 * 60

def cache_key(user_id, token):
    return f"search-ranking:{user_id}:{token}"

def save_ranking(user_id, matches):
    token = uuid4()

    cache.set(
        cache_key(user_id, token),
        matches,
        timeout=SEARCH_TTL,
    )

    return token

def get_ranking(user_id, token):
    return cache.get(cache_key(user_id, token))

def load_page(user, matches, offset=0):
    remaining = matches[offset:]

    if not remaining:
        return {
            "posts": [],
            "next_offset": None,
        }

    blocked_ids = UserBlock.objects.filter(
        blocker=user
    ).values_list("blocked_id", flat=True)

    # fetching id's here
    visible_ids = set(
        Thread.objects.filter(
            id__in=[match["id"] for match in remaining]
        )
        .exclude(author_id__in=blocked_ids)
        .values_list("id", flat=True)
    )

    eligible = [
        (position, match)
        for position, match in enumerate(remaining, start=offset)
        if match["id"] in visible_ids
    ]

    batch = eligible[:PAGE_SIZE]
    next_offset = None

    if len(eligible) > PAGE_SIZE:
        next_offset = batch[-1][0] + 1

    #only these 10 mathces are loaded as full posts
    posts = hydrate_ranked_posts([match for _, match in batch])

    return {
        "posts": posts,
        "next_offset": next_offset,
    }
