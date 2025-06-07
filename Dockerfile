############################
# 1️⃣  Build stage
############################
FROM rust:1.87-slim AS builder

# Install system build tooling **and** CBC + headers
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential pkg-config clang libclang-dev \
        # CBC + headers
        coinor-cbc coinor-libcbc-dev \
        python3 \
        # ⬇️  missing build-time deps for -lz -lbz2 -llapack -lblas
        zlib1g-dev libbz2-dev liblapack-dev libblas-dev \
        && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# First copy the manifests to leverage Docker’s layer caching
COPY Cargo.toml Cargo.lock ./

# Copy the real source and build
COPY . .
RUN cargo build --release

# Default command
ENTRYPOINT ["python3", "wrapper.py"]
# If your program requires CLI args, replace ENTRYPOINT with CMD, e.g.:
# CMD ["eggstraction", "--help"]
