#!/bin/sh

balance=$(docker exec devnet-bitcoind-electrs-1 /bitcoin/bin/bitcoin-cli -regtest -rpcwallet=alice getbalance)
echo "$balance"
