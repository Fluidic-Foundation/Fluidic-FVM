#!/usr/bin/env bash
set -euo pipefail

# Make sure gcloud is on PATH even when this script is run from a non-interactive shell.
export PATH="$PATH:$HOME/google-cloud-sdk/bin:/usr/local/google-cloud-sdk/bin:/opt/google-cloud-sdk/bin"

PROJECT_ID="${PROJECT_ID:-project-934c3e12-e0e7-4811-810}"
REGION="${REGION:-us-central1}"
REPO="fluidic"
REGISTRY="${REGION}-docker.pkg.dev/${PROJECT_ID}/${REPO}"

if command -v gcloud >/dev/null 2>&1; then
  echo "Authenticating Docker to Artifact Registry via gcloud..."
  gcloud auth configure-docker "${REGION}-docker.pkg.dev" --quiet

  # Ensure the repository exists (idempotent).
  if ! gcloud artifacts repositories describe "${REPO}" --location="${REGION}" --project="${PROJECT_ID}" >/dev/null 2>&1; then
    echo "Creating Artifact Registry repository ${REPO}..."
    gcloud artifacts repositories create "${REPO}" \
      --repository-format=docker \
      --location="${REGION}" \
      --project="${PROJECT_ID}" \
      --description="Fluidic testnet container images"
  fi
elif [[ -n "${GOOGLE_APPLICATION_CREDENTIALS:-}" ]] && [[ -f "${GOOGLE_APPLICATION_CREDENTIALS}" ]]; then
  echo "gcloud not found; authenticating Docker with service account key..."
  cat "${GOOGLE_APPLICATION_CREDENTIALS}" | docker login -u _json_key --password-stdin "https://${REGION}-docker.pkg.dev"
else
  echo "ERROR: gcloud is not installed and GOOGLE_APPLICATION_CREDENTIALS is not set."
  echo ""
  echo "Option 1: Install the Google Cloud SDK and run:"
  echo "  gcloud auth login"
  echo "  gcloud auth application-default login"
  echo "  gcloud config set project ${PROJECT_ID}"
  echo ""
  echo "Option 2: Set GOOGLE_APPLICATION_CREDENTIALS to a service account JSON key:"
  echo "  export GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json"
  echo ""
  echo "Option 3: Push from GitHub Actions using .github/workflows/deploy-testnet.yml"
  exit 1
fi

echo "Building Fluidic images and pushing to ${REGISTRY}..."

NO_CACHE_FLAG=""
if [[ "${NO_CACHE:-}" == "1" ]]; then
  NO_CACHE_FLAG="--no-cache"
fi

# mesh_node
docker build ${NO_CACHE_FLAG} -t "${REGISTRY}/mesh-node:latest" -f docker/Dockerfile .
docker push "${REGISTRY}/mesh-node:latest"

# faucet
docker build ${NO_CACHE_FLAG} -t "${REGISTRY}/faucet:latest" -f testnet/faucet/Dockerfile testnet/faucet
docker push "${REGISTRY}/faucet:latest"

# nginx gateway
docker build ${NO_CACHE_FLAG} -t "${REGISTRY}/nginx-gateway:latest" -f testnet/nginx/Dockerfile testnet/nginx
docker push "${REGISTRY}/nginx-gateway:latest"

echo "Done."
