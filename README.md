# UnTrainable

Django discussion site, Rust/C++ search, and private project hosting with Safe CLI.

## Install once

1. Install [Git](https://git-scm.com/downloads) and [Docker Desktop](https://docs.docker.com/desktop/setup/install/). On Linux, Docker Engine with the current Compose plugin also works.
2. Start Docker Desktop and wait until its engine is running. On Windows, use Linux containers and complete Docker Desktop's WSL 2 setup if prompted.

Docker installs Python, Rust, the C++ compiler, ICU, PostgreSQL, Redis, and the application dependencies for you. You do not need a local Python environment or database account. Create a local `.env` containing `SEARCH_SERVICE_TOKEN` before starting Docker.

For a fresh clone, generate a token (for example with `openssl rand -hex 32`) and
save it as `SEARCH_SERVICE_TOKEN=<generated value>` in `.env`. Keep existing `.env`
settings when adding it. Compose supplies the same token to Django and Rust; never
commit the value. Missing or empty tokens prevent startup.


## Clone and start

Open Terminal (macOS/Linux) or PowerShell (Windows):

```bash
git clone https://github.com/heavyprop/secure.git
cd secure
docker compose up --build --watch
```

If you already have the project, run the last command from the folder containing `compose.yaml`.

The first run downloads and builds dependencies, creates the database tables, and downloads the classification model. Allow several minutes and keep your internet connection available. Wait for Django's `Starting development server` message, then open **http://localhost:8000**.

Register an account on the site. Your database starts empty; cloning does not copy anyone else's users, posts, or uploads.

To populate the site with 12 sample posts across four subjects, run in a second terminal:

```bash
docker compose exec web python manage.py seed_demo
```

Posts are marked `[Sample]` and owned by `sample_content`, an account with no usable password. The command uses normal topic classification, so the first run may download the model. Rerunning skips existing sample posts. Existing posts are kept.

## Search service

Compose builds `apps/search/rust/Dockerfile` in release mode and links the C++
algorithm through CXX. Django calls `http://search:8080/search` using `SEARCH_URL`.
The search port is internal to Docker; you do not need a separate local Rust process
or Homebrew packages. Go is no longer part of the running environment.

Rebuild search and start the website in the background:

```bash
docker compose up -d --build
```

Check the service from the Django container:

```bash
docker compose exec web python manage.py shell -c 'from apps.search.client import search_with_service; result = search_with_service("hello", []); assert result is not False, "Search service unavailable"; print(result)'
```

The response contains ranked IDs and scores. Django loads display fields using
those IDs. See [Rust/C++ search](apps/search/rust/README.md) for the API and host build.

## Develop

Keep the terminal running and edit the project in your usual editor:

- Python changes reload Django automatically. Refresh the browser after HTML/CSS/JavaScript changes.
- Rust/C++ changes rebuild and restart the search service automatically. Rust fetches candidates; C++ scores them in the same process.
- Dependency changes rebuild the affected container automatically.
- Database model changes need a migration; see the commands below.

Each time you return, start Docker Desktop and run the same command:

```bash
docker compose up --build --watch
```

Press **Ctrl+C** to stop. Database records, uploaded projects, and downloaded model files remain in Docker volumes. Do not use `docker compose down -v` unless you intend to delete this stored data.

## Useful commands

Run these in a second terminal inside the project folder while the app is running:

| Task | Command |
| --- | --- |
| Create an admin account | `docker compose exec web python manage.py createsuperuser` |
| Create migrations after changing models | `docker compose exec web python manage.py makemigrations` |
| Apply migrations | `docker compose exec web python manage.py migrate` |
| Open a Django shell | `docker compose exec web python manage.py shell` |
| Check Django configuration | `docker compose exec web python manage.py check` |
| Check migration drift | `docker compose exec web python manage.py makemigrations --check --dry-run` |
| Lint Python | `docker compose exec web python -m ruff check apps common config safe_cli scripts manage.py` |
| View recent logs | `docker compose logs --tail=100 web search` |

Generated migration files are saved into your checkout. On Linux, if these files are owned by root, generate them using `docker compose exec --user "$(id -u):$(id -g)" web python manage.py makemigrations` instead.

Admin is at http://localhost:8000/admin/. For discussion-only migrations, use `makemigrations home`; `home` is the existing database app label.

## If something needs changing locally

- **Docker cannot connect:** start Docker Desktop. On Linux, make sure your account can run `docker info`.
- **`--watch` is unrecognized:** update Docker Desktop or the Compose plugin. You can temporarily use `docker compose up --build`; restart/rebuild after Rust/C++ or dependency changes.
- **A port is already in use:** stop the other application, or create a `.env` file in the project root with the overrides below. Keep any existing `.env` entries. Internal service addresses do not need changing.

  ```dotenv
  WEB_PORT=8001
  POSTGRES_PORT=5434
  REDIS_PORT=6381
  ```

  Restart the Compose command, then open http://localhost:8001 if you changed `WEB_PORT`.

- **A download fails:** check your connection and rerun the startup command. Successful image layers and model downloads are cached.
- **Docker runs out of memory or disk:** increase the resources available to Docker Desktop; ML dependencies can require several GB of disk space.
- **Your editor needs a local Python interpreter:** use the optional [host setup guide](setup.md).

`.env` is ignored by Git. Docker uses local development configuration from `compose.yaml`; `.env.example` is for the optional host setup. Compose uses `SEARCH_SERVICE_TOKEN` and the port overrides above, but does not switch its internal database connections to your host `.env` values. Existing local uploads are not imported into Docker's upload volume automatically.

This is a local development setup. Public deployment requires production settings, credentials, HTTPS, and a production web server; see [security guidance](docs/security-plan.md).

## Project guide

- [Architecture and directory map](docs/architecture.md)
- [Hosting and Safe CLI](docs/hosting.md)
- [Recommendation profiles](docs/recommendations.md)
- [Optional host setup](setup.md)
