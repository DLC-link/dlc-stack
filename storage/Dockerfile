FROM rust:1.70 AS build

COPY ./storage /app/storage-api/

WORKDIR /app/storage-api/api/
RUN rustup override set nightly-2023-03-31
RUN cargo build --release --bin storage-api

FROM debian:bullseye-slim AS dlc-link-stack
RUN apt-get update && apt-get install -y openssl libpq-dev ca-certificates && apt-get clean
COPY --from=build /app/storage-api/api/target/release/storage-api /app/storage-api
WORKDIR /app
ENTRYPOINT ["/app/storage-api"]
