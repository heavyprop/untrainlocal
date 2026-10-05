# Rust/C++ search

Rust fetches PostgreSQL candidates asynchronously with SeaQuery + SQLx. C++ scores and sorts
them through CXX on a blocking worker. Rust returns ranked IDs and scores as JSON;
Django loads display fields and filters blocked authors.

## Docker (recommended)

First set `SEARCH_SERVICE_TOKEN` in the root `.env` to a long random token.
Compose passes it to both Django and Rust. Then, from the repository root:

```bash
docker compose up -d --build
docker compose logs -f search web
```

The search service is built in release mode with Rust, a C++17 compiler and ICU.
The runtime image contains the binary and its matching ICU/C++ libraries.
No host Rust process or Homebrew packages are needed. Django uses
`SEARCH_URL=http://search:8080/search`; port 8080 is internal to Docker.
Compose waits for the search health check before starting Django.

With `docker compose up --build --watch`, edits to Rust/C++ source or Cargo files
rebuild search. Host `target/` artifacts and the reference `ALGORITHM/` are excluded.

## API

`POST /search` requires `Authorization: Bearer <SEARCH_SERVICE_TOKEN>`.
Missing, incorrect or duplicate authorization headers return `401` before the
request body is parsed or any candidate fetching/scoring runs. The token is
required at Rust and Django startup. Store it only in local configuration; a
rotation requires recreating both containers. It never goes to the browser.

`GET /health` is an unauthenticated liveness endpoint used by Docker. It returns
no application data and does not query PostgreSQL or run C++ scoring.

`POST /search` accepts:

```json
{"query":"rust", "labels":[{"id":"programming", "score":0.8}]}
```

Response shape:

```json
{"query_terms":["rust"], "results":[{"id":7, "score":0.8}]}
```

Candidate retrieval matches any query word as a case-insensitive substring in the
title/description, or a supplied positive-confidence tag ID. C++ then applies
whole-word text matching, tag overlap, engagement and recency scoring, filters
insufficient matches, and sorts by score descending and ID ascending.

The newest `SEARCH_CANDIDATE_LIMIT` matches are considered (default 2000, maximum
10000). This is a candidate cap, not pagination. Tag-only requests work; completely
empty requests return an empty result list without reading post tables.

The previous `/candidates` endpoint is no longer exposed. `ALGORITHM/` contains the
original Rust scoring reference and is not compiled. The old Python
`fetch_candidates` helper targets that retired API and is not used by the website.

## Optional local macOS build

Install a Rust toolchain and the Xcode command-line tools, then:

```bash
brew install icu4c pkg-config
export PKG_CONFIG_PATH="$(brew --prefix icu4c)/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
```

Start PostgreSQL from the repository root with `docker compose up -d postgres`.
From `apps/search/rust`:

```bash
export SEARCH_SERVICE_TOKEN='your-local-token-from-the-root-env-file'
export DATABASE_URL='postgresql://untrainable:development@127.0.0.1:5433/untrainable'
export SEARCH_LISTEN_ADDR='127.0.0.1:8081'
cargo test --locked
cargo run --release --locked
```

For Django running directly on the Mac, set `SEARCH_URL=http://127.0.0.1:8081/search`.
Both sides must use the same database. ICU is discovered through pkg-config in
`build.rs`; Docker configures this independently of your Mac.

## Database access

`repository.rs` builds a parameterized PostgreSQL statement with SeaQuery and
executes it with SQLx. `database.rs` owns the async pool (up to five connections).
No SeaORM entities are generated or required; Django owns schema migrations.

Run unit tests with `cargo test --locked`. The database integration test uses
connection-local temporary tables and can also be run against the development
PostgreSQL container:

```bash
TEST_DATABASE_URL="$DATABASE_URL" cargo test --locked -- --include-ignored
```

Set `DATABASE_URL` as shown above first. This test checks text/tag matching,
candidate limits, engagement counts and invalid stored labels without changing
application tables.
