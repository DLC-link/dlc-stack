#!/bin/sh
mkdir -p /root/.bitcoin
cp bitcoin/conf/bitcoin.conf /root/.bitcoin/bitcoin.conf
nohup bitcoin/bin/bitcoind -regtest >/dev/stdout 2>&1 &
sleep 5
bitcoin/bin/bitcoin-cli -regtest createwallet "alice"
nohup electrs -vvvv --daemon-dir /root/.bitcoin/ --network regtest --http-addr 0.0.0.0:3004 --cookie=devnet2:devnet2 --jsonrpc-import >/dev/stdout 2>&1 &
tail -f /dev/stdout
