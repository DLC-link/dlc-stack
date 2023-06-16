FROM rust:1.65 AS build

COPY Cargo.toml Cargo.lock /app/dlc-oracle/
WORKDIR /app/dlc-oracle/

RUN echo "fn main() {}" >> /app/dlc-oracle/dummy.rs
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml

COPY ./src /app/dlc-oracle/src
RUN cargo build --release

FROM debian:buster-slim AS app
RUN apt-get update && apt-get install -y openssl && apt-get clean
COPY --from=build /app/dlc-oracle/target/release/sibyls /app/sibyls
WORKDIR /app
ENTRYPOINT ["/app/sibyls"]
