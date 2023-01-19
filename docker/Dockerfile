FROM rust:1.65 AS build

COPY . /app/dlc-stack/
WORKDIR /app/dlc-stack/

RUN cargo build --release

FROM debian:buster-slim AS dlc-link-stack
RUN apt-get update && apt-get install -y openssl libpq-dev && apt-get clean
COPY --from=build /app/dlc-stack/target/release/sibyls /app/sibyls
COPY --from=build /app/dlc-stack/target/release/storage-api /app/storage-api
COPY --from=build /app/dlc-stack/target/release/dlc-link-backend /app/dlc-link-backend
WORKDIR /app
ENTRYPOINT ["/app/sibyls"]
