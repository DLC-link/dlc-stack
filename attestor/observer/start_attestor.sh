#!/bin/bash

cd "$(dirname "$0")"

. ./build_all.sh && \
npm run run
