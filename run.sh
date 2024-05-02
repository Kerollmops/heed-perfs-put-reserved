#!/usr/bin/env bash

(set -x; cargo build --release)

exe="./target/release/heed-perfs-put-reserved"

for codec in classic-codec put-reserved put-reserved-alloc put-reserved-uninit put-reserved-uninit-fill-zeroes put-reserved-uninit-into-slice; do
    # (set -x; flamegraph --root -o "flamegraphs/$codec-flamegraph.svg" -- $exe $codec)
    # (set -x; sudo perf stat -- $exe $codec)
    (set -x; $exe $codec)
done

rm -r *.mdb

