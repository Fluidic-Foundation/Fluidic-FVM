# Fluidic Testnet Rollout

This directory contains the infrastructure-as-code, deployment manifests, and operational playbooks for launching the Fluidic public testnet on Google Cloud.

## Current State

- Rust `mesh_node` binary is containerized (`docker/Dockerfile`).
- HTTP/WebSocket API serves state, balances, account registration, stateful shifts, and shift status.
- React dApp and landing site are statically built.
- Explorer and docs pages are statically built.

## Testnet Architecture

```
Users
  │
  ▼
Cloud CDN + Cloud Load Balancer (HTTPS)
  │
  ├─► Static site bucket  (testnet.fluidic.foundation)
  ├─► Nginx API Gateway   (api.testnet.fluidic.foundation)
  │     ├─► /api/*        → mesh_node pods
  │     ├─► /ws/*         → mesh_node WebSocket
  │     └─► /faucet       → faucet service
  │
  ▼
GKE Autopilot cluster
  ├─► mesh-node pods      (operator pool, 3+ replicas)
  ├─► faucet service      (test token distribution)
  └─► nginx gateway       (TLS termination, rate limiting)
```

## What is Missing vs. Required

| Component | Status | Notes |
|---|---|---|
| Docker image | ✅ Ready | `docker/Dockerfile` builds `mesh_node` |
| Persistent state | ✅ Ready | PVC per StatefulSet replica + JSON snapshot |
| Operator membership | ✅ Permissionless | Dynamic via gossiped `StakeShift`; no static registry |
| Faucet service | ✅ In this package | `faucet/` Node.js service with rate limits |
| Public DNS + TLS | ✅ Ready | GKE Ingress + ManagedCertificate |
| Monitoring / alerting | ✅ Ready | Cloud Monitoring uptime check + alert policy |
| Rate limiting / DDoS | ✅ Ready | Cloud Armor + nginx limit_req |

## Quick Start

1. Set GCP project and authenticate:
   ```bash
   gcloud auth application-default login
   gcloud config set project project-934c3e12-e0e7-4811-810
   ```

2. Provision infrastructure:
   ```bash
   cd testnet/terraform
   terraform init
   terraform apply
   ```

3. Each node derives its identity deterministically from `OSCILLATOR_ID` and
   announces its stake to the mesh via gossip. No static operator registry is
   required.

4. Build and push the node image:
   ```bash
   ./testnet/scripts/build-and-push.sh
   ```

5. Deploy to GKE:
   ```bash
   ./testnet/scripts/deploy.sh
   ```

5. Verify:
   ```bash
   curl https://api.testnet.fluidic.foundation/api/state
   ```

## Files

- `terraform/` — GKE cluster, VPC, load balancer, Cloud DNS, static IP
- `k8s/` — Kubernetes manifests for mesh nodes, faucet, nginx gateway
- `faucet/` — Test-token faucet service
- `nginx/` — Gateway configuration
- `scripts/` — Build, push, and deploy helpers
- `.github/workflows/deploy-testnet.yml` — CI/CD pipeline
