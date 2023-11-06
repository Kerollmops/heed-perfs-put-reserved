#! /bin/bash
set -ve

cargo build --release

./target/release/heed-perfs-put-reserved classic-codec
./target/release/heed-perfs-put-reserved put-reserved
./target/release/heed-perfs-put-reserved put-reserved-uninit
./target/release/heed-perfs-put-reserved put-reserved-uninit-into-slice
