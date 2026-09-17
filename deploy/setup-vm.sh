#!/usr/bin/env bash
# One-liner bootstrap for a fresh Ubuntu Server VM: installs Docker, clones
# this repo, wires up .env, and brings the stack up with `docker compose`.
#
# Usage on the VM (as a normal sudo-capable user, NOT root unless you must):
#
#   curl -fsSL https://raw.githubusercontent.com/<owner>/<repo>/main/deploy/setup-vm.sh | bash -s -- <repo-url> [branch]
#
# Example:
#   curl -fsSL https://raw.githubusercontent.com/QQUway/aiokerja/main/deploy/setup-vm.sh \
#     | bash -s -- https://github.com/QQUway/aiokerja.git main
#
# What it does:
#   1. Installs Docker Engine + Compose plugin from Docker's official apt repo
#      (idempotent — skips if already installed).
#   2. Adds the current user to the `docker` group (takes effect on next login;
#      the script itself uses `sudo docker` so it works immediately too).
#   3. Clones (or updates) the repo into ~/aiokerja.
#   4. Creates .env from .env.example if missing, and pauses so you can fill
#      in LLM_API_KEY / LLM_BASE_URL / LLM_MODEL before starting containers.
#   5. Runs `docker compose up -d --build` and waits for the backend health
#      check to go green.
set -euo pipefail

REPO_URL="${1:?usage: setup-vm.sh <repo-url> [branch] [app-dir]}"
BRANCH="${2:-main}"
APP_DIR="${3:-$HOME/aiokerja}"

log() { printf '\n\033[1;32m==> %s\033[0m\n' "$1"; }
die() { printf '\033[1;31mERROR: %s\033[0m\n' "$1" >&2; exit 1; }

[[ "$(uname -s)" == "Linux" ]] || die "this script targets Ubuntu Server (Linux) only"
command -v sudo >/dev/null || die "sudo is required"

# ---------------------------------------------------------------------------
# 1. Docker Engine + Compose plugin (official Docker repo, per docs.docker.com)
# ---------------------------------------------------------------------------
if command -v docker >/dev/null 2>&1 && sudo docker info >/dev/null 2>&1; then
  log "Docker already installed and running, skipping install"
else
  log "Installing Docker Engine"
  sudo apt-get update -y
  sudo apt-get install -y ca-certificates curl gnupg git

  sudo install -m 0755 -d /etc/apt/keyrings
  if [[ ! -f /etc/apt/keyrings/docker.asc ]]; then
    sudo curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
    sudo chmod a+r /etc/apt/keyrings/docker.asc
  fi

  ARCH="$(dpkg --print-architecture)"
  CODENAME="$(. /etc/os-release && echo "$VERSION_CODENAME")"
  echo "deb [arch=${ARCH} signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu ${CODENAME} stable" \
    | sudo tee /etc/apt/sources.list.d/docker.list >/dev/null

  sudo apt-get update -y
  sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

  sudo systemctl enable --now docker
fi

if ! groups "$USER" | grep -qw docker; then
  log "Adding $USER to the docker group (log out/in for it to apply to future shells)"
  sudo usermod -aG docker "$USER"
fi

DC() { sudo docker compose "$@"; }

# ---------------------------------------------------------------------------
# 2. Fetch / update the repo
# ---------------------------------------------------------------------------
command -v git >/dev/null 2>&1 || sudo apt-get install -y git

if [[ -d "$APP_DIR/.git" ]]; then
  log "Repo already present at $APP_DIR — pulling latest $BRANCH"
  git -C "$APP_DIR" fetch origin "$BRANCH"
  git -C "$APP_DIR" checkout "$BRANCH"
  git -C "$APP_DIR" reset --hard "origin/$BRANCH"
else
  log "Cloning $REPO_URL ($BRANCH) into $APP_DIR"
  git clone --branch "$BRANCH" "$REPO_URL" "$APP_DIR"
fi

cd "$APP_DIR"

# ---------------------------------------------------------------------------
# 3. Configure .env
# ---------------------------------------------------------------------------
if [[ ! -f .env ]]; then
  cp .env.example .env
  log "Created .env from .env.example — EDIT IT NOW to set LLM_API_KEY / LLM_BASE_URL / LLM_MODEL"
  echo "   Config file: $APP_DIR/.env"
  read -r -p "Press Enter once .env is filled in (or Ctrl+C to stop and edit manually later)... " _ || true
else
  log ".env already exists, leaving it as-is"
fi

# ---------------------------------------------------------------------------
# 4. Build and start
# ---------------------------------------------------------------------------
log "Building and starting containers"
DC up -d --build

log "Waiting for backend health check"
BACKEND_PORT="$(grep -E '^BACKEND_PORT=' .env | cut -d= -f2)"
BACKEND_PORT="${BACKEND_PORT:-8080}"
ok=0
for i in $(seq 1 40); do
  if curl -fsS -m 2 "http://127.0.0.1:${BACKEND_PORT}/health" >/dev/null 2>&1; then
    ok=1
    break
  fi
  sleep 3
done

if [[ "$ok" == "1" ]]; then
  log "Backend healthy at http://$(curl -fsS ifconfig.me 2>/dev/null || echo "<vm-ip>"):${BACKEND_PORT}/health"
else
  echo "Backend did not become healthy in time — check logs with:"
  echo "  cd $APP_DIR && sudo docker compose logs -f backend"
  exit 1
fi

FRONTEND_PORT=5173
log "Done. Frontend: http://$(curl -fsS ifconfig.me 2>/dev/null || echo "<vm-ip>"):${FRONTEND_PORT}"
echo
echo "IMPORTANT: this app has no authentication. Do not expose ports ${BACKEND_PORT}/${FRONTEND_PORT}"
echo "to the public internet — put it behind a VPN, Tailscale, or an authenticating reverse proxy."
echo
echo "Manage it with:"
echo "  cd $APP_DIR && sudo docker compose ps"
echo "  cd $APP_DIR && sudo docker compose logs -f"
echo "  cd $APP_DIR && sudo docker compose down"
