#!/bin/bash

mkdir -p target

start_storage() {
  RUST_LOG=debug DATABASE_URL=postgresql://postgres:changeme@localhost:5432/postgres MIGRATE=true cargo run --bin $storage_api_app > target/$storage_api_app.log 2> target/$storage_api_app.log &
  echo $! > $storage_api_pid_file
  while ! nc -z localhost $storage_api_port; do
    echo "Retry to access $storage_api_app on port $storage_api_port in 5 sec ..."
    sleep 5
  done
  echo "$storage_api_app started with PID $(cat $storage_api_pid_file)"
}

start_oracle() {
  export STORAGE_API_ENDPOINT=http://localhost:8100
  RUST_LOG=debug cargo run --bin $oracle_app > target/$oracle_app.log 2> target/$oracle_app.log &
  echo $! > $oracle_pid_file
  while ! nc -z localhost $oracle_port; do
    echo "Retry to access $oracle_app on port $oracle_port in 5 sec ..."
    sleep 5
  done
  echo "$oracle_app started with PID $(cat $oracle_pid_file)"
}

start_wallet() {
  export BTC_RPC_URL="electrs-btc2.dlc.link:18443/wallet/alice"
  export RPC_USER="devnet2"
  export RPC_PASS="devnet2"
  export ORACLE_URL=http://localhost:8080
  export STORAGE_API_ENDPOINT=http://localhost:8100
  RUST_LOG=debug cargo run --bin $wallet_app > target/$wallet_app.log 2> target/$wallet_app.log &
  echo $! > $wallet_pid_file
  while ! nc -z localhost $wallet_port; do
    echo "Retry to access $wallet_app on port $wallet_port in 5 sec ..."
    sleep 5
  done
  echo "$wallet_app started with PID $(cat $wallet_pid_file)"
}

storage_api_app="storage-api"
storage_api_port="8100"
storage_api_pid_file="target/$storage_api_app.pid"

if [[ -f $storage_api_pid_file ]]; then
  pid=$(cat $storage_api_pid_file)
  if ps -p $pid > /dev/null; then
    echo "$storage_api_app has been already started with PID $(cat $storage_api_pid_file)"
  else
    start_storage
  fi
else
  start_storage
fi

oracle_app="sibyls"
oracle_port="8080"
oracle_pid_file="target/$oracle_app.pid"
#oracle_params="--asset-pair-config-file oracle/config/asset_pair.json --oracle-config-file oracle/config/oracle.json --secret-key-file oracle/config/secret.key"

if [[ -f $oracle_pid_file ]]; then
  pid=$(cat $oracle_pid_file)
  if ps -p $pid > /dev/null; then
    echo "$oracle_app has been already started with PID $(cat $oracle_pid_file)"
  else
    start_oracle
  fi
else
  start_oracle
fi

wallet_app="dlc-protocol-wallet"
wallet_port="8085"
wallet_pid_file="target/$wallet_app.pid"

if [[ -f $wallet_pid_file ]]; then
  pid=$(cat $wallet_pid_file)
  if ps -p $pid > /dev/null; then
    echo "$wallet_app has been already started with PID $(cat $wallet_pid_file)"
  else
    start_wallet
  fi
else
  start_wallet
fi
