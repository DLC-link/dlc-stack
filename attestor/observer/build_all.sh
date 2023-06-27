#!/bin/bash

# Change to the directory of the current script
cd "$(dirname "$0")"

# Building wasm in the folder above
AR=/opt/homebrew/opt/llvm/bin/llvm-ar CC=/opt/homebrew/opt/llvm/bin/clang wasm-pack build --target bundler ..

# Rewriting the attestor's package.json to be an ES module
# https://github.com/gthb/try-to-use-wasm-in-next.js/blob/main/package.json
jq '. + {type: "module", main: "attestor.js"} | del(.module)' ../pkg/package.json > temp.json
mv temp.json ../pkg/package.json

# Adding the crypto shim
echo 'import { webcrypto } from "node:crypto"; globalThis.crypto = webcrypto;' >> ../pkg/attestor_bg.js

#  Compiling typescript
npx tsc -p .

# Reinstalling updated attestor pkg
npm ci attestor
