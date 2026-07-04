FROM rust:1.92-slim AS builder
WORKDIR /usr/src/fluidic

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY genesis ./genesis
RUN CARGO_BUILD_JOBS=1 cargo build --release --bin mesh_node

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates iputils-ping && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/src/fluidic/target/release/mesh_node /app/mesh_node
EXPOSE 7000 8080
# Use CMD (not ENTRYPOINT) so Railway's start command overrides the default
# command cleanly instead of being passed as arguments to mesh_node.
CMD ["/app/mesh_node"]
