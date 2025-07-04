#!/usr/bin/env bash
set -e

./scripts/build_images.sh
./scripts/deploy.sh
./scripts/restart.sh