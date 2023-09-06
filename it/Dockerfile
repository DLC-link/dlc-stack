FROM cimg/rust:1.71.0-node AS it-build

USER circleci

# Install necessary dependencies and wasm-pack, and clean up
RUN sudo apt-get update && \
  sudo apt-get install -y curl jq bash build-essential clang && \
  cargo install wasm-pack && \
  sudo apt-get clean

COPY ./clients/ /app/clients
COPY ./attestor-client /app/attestor-client
COPY ./dlc-link-manager/ /app/dlc-link-manager
COPY ./js-interface-wallet/ /app/js-interface-wallet
COPY ./esplora-async-blockchain-provider/ /app/esplora-async-blockchain-provider
COPY ./wasm-wallet/ /app/wasm-wallet

COPY ./it/ /app/it

WORKDIR /app

RUN rustup override set nightly
RUN sudo chown -R circleci:circleci /app
RUN wasm-pack build --target bundler /app/wasm-wallet

WORKDIR /app/it

RUN jq '. + {type: "module", main: "dlc-wasm-wallet.js"} | del(.module)' ../wasm-wallet/pkg/package.json > temp.json && \
  mv temp.json ../wasm-wallet/pkg/package.json && \
  echo 'import { webcrypto } from "node:crypto"; globalThis.crypto = webcrypto;' >> ../wasm-wallet/pkg/dlc_wasm_wallet_bg.js


RUN npm install && \
  sudo npm ci wasm-wallet


FROM node:16-alpine AS it

COPY --from=it-build /app/it/src/index.js /app/it/src/index.js
COPY --from=it-build /app/it/src/config.js /app/it/src/config.js
COPY --from=it-build /app/it/src/polyfills.js /app/it/src/polyfills.js
COPY --from=it-build /app/it/package.json /app/it/package.json
COPY --from=it-build /app/it/node_modules /app/it/node_modules
COPY --from=it-build /app/wasm-wallet/pkg /app/wasm-wallet/pkg

WORKDIR /app/it

CMD npm start
