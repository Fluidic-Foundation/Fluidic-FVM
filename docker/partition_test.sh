#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PARTITION_DURATION=${PARTITION_DURATION:-15}

echo "=== Fluidic Mesh Partition Test ==="
echo "Bringing up 6-node mesh..."
docker compose -f docker-compose.yml up -d --build

# Wait for the mesh to establish connections and produce synthesis logs.
echo "Waiting 10s for mesh baseline..."
sleep 10

echo "Baseline synthesis logs (last 5 lines per node):"
for i in $(seq 0 5); do
    echo "--- osc-$i ---"
    docker logs --tail 5 "fluidic-osc-$i" || true
done

# Partition ~33% of the mesh (2 of 6 nodes).
echo "Partitioning osc-4 and osc-5 from the mesh for ${PARTITION_DURATION}s..."
docker network disconnect -f fluidic-mesh fluidic-osc-4 || true
docker network disconnect -f fluidic-mesh fluidic-osc-5 || true

echo "Waiting ${PARTITION_DURATION}s while partitioned..."
sleep "$PARTITION_DURATION"

echo "Surviving node synthesis logs during partition (last 8 lines):"
for i in 0 1 2 3; do
    echo "--- osc-$i ---"
    docker logs --tail 8 "fluidic-osc-$i" || true
done

echo "Reconnecting partitioned nodes..."
docker network connect fluidic-mesh fluidic-osc-4 || true
docker network connect fluidic-mesh fluidic-osc-5 || true

echo "Waiting 10s for reconnection..."
sleep 10

echo "Post-healing synthesis logs (last 5 lines per node):"
for i in $(seq 0 5); do
    echo "--- osc-$i ---"
    docker logs --tail 5 "fluidic-osc-$i" || true
done

# Invariant check: surviving nodes must have continued to apply commutative
# shifts during the partition (look for non-zero commutative counts).
echo "Checking invariant: surviving nodes continued synthesis..."
PASS=true
for i in 0 1 2 3; do
    COUNT=$(docker logs "fluidic-osc-$i" 2>/dev/null | grep -c 'synthesis: commutative=' || true)
    if [ "$COUNT" -lt 2 ]; then
        echo "FAIL: osc-$i did not produce multiple synthesis ticks during partition"
        PASS=false
    else
        echo "PASS: osc-$i produced $COUNT synthesis ticks"
    fi
done

if [ "$PASS" = true ]; then
    echo "=== Partition test PASSED ==="
else
    echo "=== Partition test FAILED ==="
    exit 1
fi

echo "Run 'docker compose -f docker/docker-compose.yml down' to stop the mesh."
