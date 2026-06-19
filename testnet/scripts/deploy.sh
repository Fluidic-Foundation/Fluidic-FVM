#!/usr/bin/env bash
set -euo pipefail

NAMESPACE="fluidic-testnet"
K8S_DIR="testnet/k8s"

echo "Deploying Fluidic testnet to Kubernetes..."

kubectl create namespace "${NAMESPACE}" --dry-run=client -o yaml | kubectl apply -f -

kubectl apply -f "${K8S_DIR}/namespace.yaml"
kubectl apply -f "${K8S_DIR}/mesh-node.yaml"
kubectl apply -f "${K8S_DIR}/faucet.yaml"
kubectl apply -f "${K8S_DIR}/nginx.yaml"
kubectl apply -f "${K8S_DIR}/gateway.yaml"

echo "Waiting for mesh-node rollout..."
kubectl rollout status statefulset/mesh-node -n "${NAMESPACE}" --timeout=180s

echo "Waiting for faucet rollout..."
kubectl rollout status deployment/faucet -n "${NAMESPACE}" --timeout=180s

echo "Waiting for nginx-gateway rollout..."
kubectl rollout status deployment/nginx-gateway -n "${NAMESPACE}" --timeout=180s

echo ""
echo "Testnet deployed. Ingress IP will be provisioned by GCE for api.testnet.fluidic.foundation"
kubectl get ingress fluidic-api-ingress -n "${NAMESPACE}" -o jsonpath='{.status.loadBalancer.ingress[0].ip}'
echo ""
