#!/bin/bash

# Save the current working directory
cwd=$(pwd)

# Change to the directory above the current script
cd "$(dirname "$0")/.."

npm ci

# Building wasm in the wasm-wallet folder
AR=/opt/homebrew/opt/llvm/bin/llvm-ar CC=/opt/homebrew/opt/llvm/bin/clang wasm-pack build --target bundler ../wasm-wallet

# Rewriting the wasm pkg's package.json to be an ES module
# https://github.com/gthb/try-to-use-wasm-in-next.js/blob/main/package.json
jq '. + {type: "module", main: "dlc-tools.js"} | del(.module)' ../wasm-wallet/pkg/package.json > temp.json
mv temp.json ../wasm-wallet/pkg/package.json

# Adding the crypto shim
echo 'import { webcrypto } from "node:crypto"; globalThis.crypto = webcrypto;' >> ../wasm-wallet/pkg/dlc_tools_bg.js

#  Compiling typescript
# npx tsc -p .

npm ci wasm-wallet

# Return to the original working directory
cd "$cwd"
