#!/bin/bash

# see https://crates.io/crates/cargo-instruments

input="${1:?Missing input file}"

cargo instruments -t "CPU Profiler" --bin pbrt -- \
   --input "${input}"
  --output "output_dev.png" \
  --max_depth 5 \
  --samples_ppx 5 \
  --output_width 800 \
  --output_height 600 \
  --threads 1
