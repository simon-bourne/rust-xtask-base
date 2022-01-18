#!/usr/bin/env sh

set -eu

cargo run -p project-workflow -- "$@"
