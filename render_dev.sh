#!/bin/bash

time cargo run --release -- --output "output_dev.png" \
  --max_depth 6 \
  --samples_ppx 5 \
  --output_width 1067 \
  --output_height 600 \
  --threads 1
