FROM rust:latest

# Install prerequisites
RUN apt-get update && apt-get install -y \
  libasound2-dev \
  libudev-dev \
  binaryen \
  && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli --locked

# Copy base project
WORKDIR /usr/src/app
RUN cargo init --name game --vcs none
COPY ./Cargo.toml ./Cargo.toml
COPY ./.cargo ./.cargo

# Build deps
RUN cargo fetch
RUN cargo build --release

# Clean up
RUN rm src/*.rs
RUN rm ./target/wasm32-unknown-unknown/release/deps/game*
