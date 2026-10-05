from django.urls import path

from .views import load_more, search

urlpatterns = [
    path("", search, name="search"),
    path("more/<uuid:token>/", load_more, name="search_load_more"),
]
