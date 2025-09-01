FROM rust:1.89.0-slim-trixie AS build

WORKDIR /app

RUN apt-get update && apt-get -y upgrade && apt-get install -y build-essential pkg-config libssl-dev

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --locked --release
cp ./target/release/nash-stats /bin/nash-stats
EOF

FROM debian:trixie-slim AS final

RUN apt-get update && apt-get -y upgrade && apt-get install -y ca-certificates

COPY --from=build /bin/nash-stats /bin/

CMD ["/bin/nash-stats"]