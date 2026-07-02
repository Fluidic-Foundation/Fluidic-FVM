#!/bin/bash
set -e

# Fluidic mesh node one-liner installer for Linux/macOS.
# Tries Docker first, then falls back to building from source.

IMAGE="ghcr.io/fluidic-foundation/fluidic-fvm/mesh-node:latest"
DATA_DIR="${FLUIDIC_DATA_DIR:-$HOME/fluidic-data}"
OSCILLATOR_ID="${OSCILLATOR_ID:-0}"
API_PORT="${API_PORT:-8080}"
BIND_ADDR="${BIND_ADDR:-0.0.0.0:7000}"
PEERS="${PEERS:-}"

if command -v docker &> /dev/null; then
    echo "Running Fluidic mesh node with Docker..."
    docker run -d --name fluidic-node \
        --restart unless-stopped \
        -p "${API_PORT}:${API_PORT}" \
        -p "${BIND_ADDR#*:}:7000" \
        -e OSCILLATOR_ID="${OSCILLATOR_ID}" \
        -e API_PORT="${API_PORT}" \
        -e BIND_ADDR="${BIND_ADDR}" \
        -e PEERS="${PEERS}" \
        -e FLUIDIC_DATA_DIR=/data \
        -v "${DATA_DIR}:/data" \
        "${IMAGE}"
    echo "Node started. Logs: docker logs -f fluidic-node"
    exit 0
fi

if ! command -v cargo &> /dev/null; then
    echo "Docker not found and Rust/Cargo not installed."
    echo "Install Rust: https://rustup.rs/"
    exit 1
fi

REPO_URL="https://github.com/Fluidic-Foundation/Fluidic-FVM.git"
INSTALL_DIR="${INSTALL_DIR:-$HOME/fluidic-fvm}"

if [ ! -d "${INSTALL_DIR}" ]; then
    git clone "${REPO_URL}" "${INSTALL_DIR}"
fi

cd "${INSTALL_DIR}"
cargo build --release --bin mesh_node

FLUIDIC_DATA_DIR="${DATA_DIR}" \
OSCILLATOR_ID="${OSCILLATOR_ID}" \
API_PORT="${API_PORT}" \
BIND_ADDR="${BIND_ADDR}" \
PEERS="${PEERS}" \
./target/release/mesh_node
