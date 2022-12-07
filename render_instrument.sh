#!/bin/bash

# see https://crates.io/crates/cargo-instruments

cargo instruments -t "CPU Profiler" --bin pbrt -- \
  --output "output_dev.png" \
  --max_depth 5 \
  --samples_ppx 5 \
  --output_width 800 \
  --output_height 600 \
  --threads 1
