#!/bin/bash

time cargo run --release -- --output "output.png" --max_depth 5 --samples_ppx 1000 --output_width 1920 --output_height 1080 --fov 20 --threads 8 --lens_radius 0.15 --focal_distance 13.5
