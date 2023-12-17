#!/bin/sh

cd runner
cargo build
cargo run -- "$1"
