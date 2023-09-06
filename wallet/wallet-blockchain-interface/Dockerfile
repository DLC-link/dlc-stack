# Build blockchain-interface NodeJS project
FROM node:16-alpine AS BI-build

WORKDIR /app/blockchain-interface

COPY ./wallet/wallet-blockchain-interface/package*.json ./
RUN npm install
COPY ./wallet/wallet-blockchain-interface/ ./
RUN npm run build
EXPOSE 3000
CMD [ "node", "dist/src/index.js" ]
