#!/bin/bash

input="${1:?Missing input file}"

time cargo run --release -- --input "${input}" --output "output.png" \
  --far 2000 \
  --max_depth 20 \
  --samples_ppx 2500 \
  --output_width 1920 \
  --output_height 1080 \
  --threads 8
#  --fov 20 \
#  --lens_radius 0.15 \
#  --focal_distance 13.5

