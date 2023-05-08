#!/bin/bash

input="${1:?Missing input file}"

time cargo run --release -- --input "${input}" --output "output_dev.png" \
  --max_depth 5 \
  --far 2000 \
  --samples_ppx 5 \
  --output_width 800 \
  --output_height 600 \
  --threads 1 \
  --integrator normal
