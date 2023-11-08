# Use cimg/rust:1.71.0-node as the base image
FROM cimg/rust:1.71.0-node AS attestor-build

USER circleci

# Install necessary dependencies and wasm-pack, and clean up
RUN sudo apt-get update && \
  sudo apt-get install -y curl jq bash build-essential clang && \
  cargo install wasm-pack && \
  sudo apt-get clean

COPY ./clients/ /app/clients
COPY ./dlc-link-manager/ /app/dlc-link-manager

COPY ./attestor/observer/src /app/attestor/observer/src
COPY ./attestor/observer/package.json /app/attestor/observer/package.json
COPY ./attestor/observer/tsconfig.json /app/attestor/observer/tsconfig.json
COPY ./attestor/src /app/attestor/src
COPY ./attestor/Cargo.toml /app/attestor/Cargo.toml

WORKDIR /app/attestor

RUN rustup override set nightly-2023-10-23
RUN sudo chown -R circleci:circleci /app
RUN wasm-pack build --target bundler .

RUN jq '. + {type: "module", main: "attestor.js"} | del(.module)' ./pkg/package.json > temp.json && \
  mv temp.json ./pkg/package.json && \
  echo 'import { webcrypto } from "node:crypto"; globalThis.crypto = webcrypto;' >> ./pkg/attestor_bg.js

WORKDIR /app/attestor/observer
RUN npm install && \
  npx tsc -p . && \
  sudo npm ci attestor

FROM node:18-alpine AS attestor

WORKDIR /app/attestor

COPY --from=attestor-build /app/attestor/observer/dist /app/attestor/observer/dist
COPY --from=attestor-build /app/attestor/observer/package.json /app/attestor/observer/package.json
COPY --from=attestor-build /app/attestor/observer/tsconfig.json /app/attestor/observer/tsconfig.json
COPY --from=attestor-build /app/attestor/observer/node_modules /app/attestor/observer/node_modules
COPY --from=attestor-build /app/attestor/pkg /app/attestor/pkg

WORKDIR /app/attestor/observer

CMD if [ "$NODE_ENV" = "development" ] ; then npm run dev ; else npm run run ; fi
