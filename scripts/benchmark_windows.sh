#!/usr/bin/env bash
set -euo pipefail

ISO="${1:-}"
DEVICE="${2:-}"
ITERATIONS="${3:-3}"

if [[ -z "$ISO" || -z "$DEVICE" ]]; then
  echo "Usage: $0 <iso_path> <device> [iterations]" >&2
  exit 1
fi

echo "Benchmarking Windows USB creation on $DEVICE with ISO $ISO"
echo "Iterations: $ITERATIONS"

for i in $(seq 1 "$ITERATIONS"); do
  echo "=== Run $i / $ITERATIONS ==="
  sudo wipefs -a "$DEVICE"
  start_ts=$(date +%s)
  if ! sudo ./target/release/majusb write --iso "$ISO" --device "$DEVICE"; then
    echo "Run $i failed" >&2
    exit 1
  fi
  end_ts=$(date +%s)
  duration=$((end_ts - start_ts))
  echo "Run $i duration: ${duration}s"
  echo "Sleeping 5s before next run..."
  sleep 5
done

echo "Benchmark complete."
