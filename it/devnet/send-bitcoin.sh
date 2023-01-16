#!/bin/sh

: "${STACKS_ALL_LOCATION:?STACKS_ALL_LOCATION is not set}"

times=$1
bitcoin_address=$2
if [[ "$times" == "" ]]; then
  n_times=1
else
  n_times=$((times))
fi
if [[ "$bitcoin_address" == "" ]]; then
  bitcoin_address=$(docker exec devnet-bitcoind-electrs-1 /bitcoin/bin/bitcoin-cli -regtest -rpcwallet=alice getnewaddress "legacy")
fi
bitcoin_address=$(echo "$bitcoin_address" | tr '[:upper:]' '[:lower:]')
echo "Address to use: $bitcoin_address"

cat <<EOT >> $STACKS_ALL_LOCATION/deployments/send.yml
---
id: 1
name: Devnet deployment
network: devnet
stacks-node: "http://localhost:20443"
bitcoin-node: "http://devnet:devnet@localhost:18443"
plan:
  batches:
      - id: 0
        transactions:
          - btc-transfer:
              expected-sender: mjSrB3wS4xab3kYqFktwBzfTdPg367ZJ2d
              recipient: ${bitcoin_address}
              sats-amount: 100000000
              sats-per-byte: 10
EOT

for (( i=1; i<=$n_times; i++ ))
do
	sh -c "cd $STACKS_ALL_LOCATION && echo "Y" | clarinet deployments apply -p deployments/send.yml"
done

rm $STACKS_ALL_LOCATION/deployments/send.yml
