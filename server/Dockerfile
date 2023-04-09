FROM rust:latest as BUILDER
WORKDIR /usr/src/app

# Copy manifests
RUN cargo init --name temp
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Build deps
RUN cargo build --release
RUN rm src/*.rs

# Copy src
COPY ./src ./src

# Build app
RUN rm ./target/release/deps/compiler*
RUN cargo build --release

# Rust not needed to run server
FROM docker:latest
WORKDIR /usr/src/app

# Install docker
# RUN curl -sSL https://get.docker.com/ | sh

# Setup ports
ENV PORT=8080
EXPOSE 8080

# Copy executable
COPY --from=BUILDER /usr/src/app/target/release/compiler .

# Copy app
COPY ./app ./app

# Run the App
# CMD ["./compiler"]
