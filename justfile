set dotenv-load := true

default:
  just --list

run-wallet:
  cd wallet && ATTESTOR_CONFIG=local TEST_MODE_ENABLED=true just run

run-attestors:
  cd attestor && just run-multiple

run-storage:
  cd storage && just run

build-wasm:
  cd it && npm run build

the-script:
  ENV=devnet HANDLE_ATTESTORS=false npm start && wait

alias it := run
alias run-it := run
alias doit := run
alias do-it := run

run:
  just build-wasm && wait
  just run-storage &
  just run-attestors &
  just run-wallet &
  while ! nc -z localhost 8085; do sleep 1; done
  just die

die:
  lsof -t -i:3000 2>/dev/null | xargs -r kill
  lsof -t -i:3001 2>/dev/null | xargs -r kill
  lsof -t -i:8801 2>/dev/null | xargs -r kill
  lsof -t -i:8802 2>/dev/null | xargs -r kill
  lsof -t -i:8803 2>/dev/null | xargs -r kill
  lsof -t -i:8100 2>/dev/null | xargs -r kill
  lsof -t -i:8085 2>/dev/null | xargs -r kill
