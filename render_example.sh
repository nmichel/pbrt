#!/bin/bash

target="${1:-cornel_box}"
output="${target}_dev.png"
time cargo run --example $target -- --output images/$output \
  --max_depth 20 \
  --far 2000 \
  --samples_ppx 200 \
  --output_width 800 \
  --output_height 600 \
  --threads 8
