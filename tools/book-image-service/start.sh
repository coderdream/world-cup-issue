#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${BOOK_IMAGE_SERVICE_DIR:-/Volumes/System/docker/book-image-service}"
PORT="${BOOK_IMAGE_PORT:-30019}"

cd "$APP_DIR"

export HF_HUB_OFFLINE="${HF_HUB_OFFLINE:-1}"
export HF_ENDPOINT="${HF_ENDPOINT:-https://hf-mirror.com}"
export BOOK_IMAGE_DTYPE="${BOOK_IMAGE_DTYPE:-auto}"
export BOOK_IMAGE_MODEL="${BOOK_IMAGE_MODEL:-Lykon/dreamshaper-8-lcm}"
export BOOK_IMAGE_OUTPUT_DIR="${BOOK_IMAGE_OUTPUT_DIR:-$APP_DIR/outputs}"

exec "$APP_DIR/.venv/bin/python" -m uvicorn app:app --host 0.0.0.0 --port "$PORT"
