#! /bin/bash
set -ve

cargo build --release

rm -rf data.mdb
./target/release/heed-perfs-put-reserved classic-codec

rm -rf data.mdb
./target/release/heed-perfs-put-reserved put-reserved
