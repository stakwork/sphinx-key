# Rust as the base image
FROM rust:latest as build

RUN rustup toolchain install nightly

RUN rustup component add rustfmt --toolchain nightly-x86_64-unknown-linux-gnu

# Create a new empty shell project
RUN USER=root cargo new --bin sphinx-key-broker
WORKDIR /sphinx-key-broker

# Copy parser dep
COPY ../parser ../parser

# Copy our manifests
COPY ./broker/Cargo.lock ./Cargo.lock
COPY ./broker/Cargo.toml ./Cargo.toml

# Build only the dependencies to cache them
RUN cargo +nightly build --release
RUN rm src/*.rs

# Copy the source code
COPY ./broker/src ./src

# Build for release.
RUN rm ./target/release/deps/sphinx_key_broker*
RUN cargo +nightly build --release

# The final base image
FROM debian:bullseye-slim

# get root CA certs
# RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates

# Copy from the previous build
COPY --from=build /sphinx-key-broker/target/release/sphinx-key-broker /usr/src/sphinx-key-broker
# COPY --from=build /sphinx-key-broker/target/release/sphinx-key-broker/target/x86_64-unknown-linux-musl/release/sphinx-key-broker .

# rocket
ENV ROCKET_ADDRESS=0.0.0.0
EXPOSE 8000

# Run the binary
# CMD ["/usr/src/sphinx-key-broker"]