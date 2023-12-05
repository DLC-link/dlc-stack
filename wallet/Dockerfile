FROM rustlang/rust:nightly-bullseye AS build

# Build Rust app
COPY ./wallet/src /app/dlc-protocol-wallet/src
COPY ./wallet/Cargo.toml /app/dlc-protocol-wallet/Cargo.toml
COPY ./dlc-link-manager /app/dlc-link-manager/
COPY ./clients /app/clients/
COPY ./attestor-client /app/attestor-client/
COPY ./dlc-wallet /app/dlc-wallet/
COPY ./esplora-async-blockchain-provider-js-wallet /app/esplora-async-blockchain-provider-js-wallet/
COPY ./esplora-async-blockchain-provider-router-wallet /app/esplora-async-blockchain-provider-router-wallet/
COPY ./js-interface-wallet /app/js-interface-wallet/

WORKDIR /app/dlc-protocol-wallet/
RUN cargo build --release --bin dlc-protocol-wallet


# Build Node.js app
FROM node:18-alpine AS BI-build

WORKDIR /app/blockchain-interface

COPY ./wallet/package.json ./package.json
RUN npm install
COPY ./wallet/wallet-blockchain-interface/src ./wallet-blockchain-interface/src
COPY ./wallet/tsconfig.json ./tsconfig.json
RUN npm run build


# Combine both services
FROM cimg/rust:1.71.0-node
USER root

# Install required packages
RUN apt-get update && apt-get install -y openssl libpq-dev ca-certificates curl supervisor && apt-get clean

COPY --from=build /usr/lib/aarch64-linux-gnu/libssl.so.1.1 /usr/lib/aarch64-linux-gnu/libssl.so.1.1
COPY --from=build /usr/lib/aarch64-linux-gnu/libcrypto.so.1.1 /usr/lib/aarch64-linux-gnu/libcrypto.so.1.1

COPY --from=build /app/dlc-protocol-wallet/target/release/dlc-protocol-wallet /app/dlc-protocol-wallet
COPY --from=BI-build /app/blockchain-interface /app/blockchain-interface

COPY ./wallet/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

WORKDIR /app

CMD ["supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
