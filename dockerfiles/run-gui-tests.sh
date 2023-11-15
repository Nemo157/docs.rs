#!/usr/bin/env bash

set -euo pipefail

run() {
  cmd="$1"
  shift
  printf '     \e[36;1mRunning\e[0m `%q' "$cmd" >&2
  printf ' %q' "$@" >&2
  printf '`\n' >&2
  "$cmd" "$@"
}

# Pre-build the images for the web server and test containers
run docker compose build gui-tests-web gui-tests

# Stop any leftover server from prior runs
run docker compose stop gui-tests-web

# Ensure the database and S3 are running
run docker compose up --wait --wait-timeout 10 db s3

# Wipe the database and S3 storage
run docker compose exec db dropdb --user cratesfyi gui-tests
run docker compose exec db createdb --user cratesfyi gui-tests
run docker compose exec s3 rm -rf /data/gui-tests
run docker compose exec s3 mkdir -p /data/gui-tests

# Add the information we need
run docker compose run --rm gui-tests-web database migrate
run docker compose run --rm gui-tests-web build update-toolchain
run docker compose run --rm gui-tests-web build crate sysinfo 0.23.4
run docker compose run --rm gui-tests-web build crate sysinfo 0.23.5

# Start the web server up
run docker compose up --wait --wait-timeout 10 gui-tests-web
trap 'run docker compose stop gui-tests-web' EXIT

# Run the tests
run docker compose run gui-tests
