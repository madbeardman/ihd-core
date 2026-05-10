#!/bin/bash
set -e

PI_HOST="colinr@home-ihd.local"
PI_APP_DIR="~/agile-fetcher"

FORCE_ENV=false
if [[ "$1" == "--force-env" ]]; then
  FORCE_ENV=true
fi

echo "==> Syncing project to Pi..."
rsync -avz \
  --exclude target \
  --exclude .git \
  --exclude node_modules \
  --exclude .env \
  --exclude deploy-pi.sh \
  ./ "${PI_HOST}:${PI_APP_DIR}"

echo "==> Handling .env file..."

if $FORCE_ENV; then
  echo "==> Forcing .env overwrite..."
  scp .env "${PI_HOST}:${PI_APP_DIR}/.env"
else
  echo "==> Checking if .env exists on Pi..."

  ssh "${PI_HOST}" "[ -f ${PI_APP_DIR}/.env ]" || {
    echo "==> .env missing on Pi, copying..."
    scp .env "${PI_HOST}:${PI_APP_DIR}/.env"
  }

  echo "==> .env left unchanged (use --force-env to overwrite)"
fi

echo "==> Building on Pi..."
ssh "${PI_HOST}" "
  source \$HOME/.cargo/env &&
  cd ${PI_APP_DIR} &&
  cargo build --release
"

echo "==> Restarting IHD device..."
ssh "${PI_HOST}" "
  sudo reboot
"

echo "==> Done."