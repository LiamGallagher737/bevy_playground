FROM rust:latest

# Install prerequisites
RUN apt-get update && apt-get install -y \
  libasound2-dev \
  libudev-dev \
  binaryen \
  && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown

# Copy Base Project
WORKDIR /usr/src/app
COPY . .

# Build deps
RUN cargo fetch
RUN cargo build --release

# Build the app
CMD cargo build --release \
  && mv target/wasm32-unknown-unknown/release/app.wasm src/app.wasm
