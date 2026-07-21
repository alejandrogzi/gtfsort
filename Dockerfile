# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.94.0
ARG DEBIAN_VERSION=bookworm-slim

FROM rust:${RUST_VERSION}-bookworm AS builder

WORKDIR /workspace

# Keep dependency resolution reproducible and preserve the crate's relative
# README path without copying the rest of the repository into the build image.
COPY README.md LICENSE ./
COPY gtfsort/Cargo.toml gtfsort/Cargo.lock ./gtfsort/
COPY gtfsort/build.rs gtfsort/cbindgen.toml gtfsort/cbindgen_cxx.toml ./gtfsort/
COPY gtfsort/include/ ./gtfsort/include/
COPY gtfsort/src/ ./gtfsort/src/

RUN cargo build \
        --locked \
        --release \
        --manifest-path gtfsort/Cargo.toml \
        --bin gtfsort \
    && strip gtfsort/target/release/gtfsort

FROM debian:${DEBIAN_VERSION} AS runtime

LABEL org.opencontainers.image.title="gtfsort" \
      org.opencontainers.image.description="Fast chromosome, position, and feature sorter for GTF/GFF annotations" \
      org.opencontainers.image.source="https://github.com/alejandrogzi/gtfsort" \
      org.opencontainers.image.licenses="MIT"

COPY --from=builder /workspace/gtfsort/target/release/gtfsort /usr/local/bin/gtfsort

WORKDIR /data

ENTRYPOINT ["/usr/local/bin/gtfsort"]
CMD ["--help"]
