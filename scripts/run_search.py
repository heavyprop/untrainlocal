"""Run Rust/C++ search with the same .env database configuration as Django."""

import subprocess

from config.environment import BASE_DIR, load_environment, required


def main():
    load_environment()
    required("DATABASE_URL")
    required("SEARCH_SERVICE_TOKEN")
    return subprocess.call(
        ["cargo", "run", "--release", "--locked"], cwd=BASE_DIR / "apps/search/rust"
    )


if __name__ == "__main__":
    raise SystemExit(main())
