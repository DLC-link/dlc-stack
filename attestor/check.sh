#!/bin/bash

uuids=$(jq '.[] | select(.rust_attestation == null) | .event_id' events.json | sed 's/^"//;s/"$//')

uuids=$(grep -vxFf nonstx.txt <<< "$uuids")

for uuid in $uuids; do
  curl localhost:8801/force-check/$uuid
  curl localhost:8802/force-check/$uuid
  curl localhost:8803/force-check/$uuid
# lets sleep for 2 seconds to avoid overloading the attestors
  sleep 2
done
