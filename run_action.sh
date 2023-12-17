#!/bin/sh

ls
cd packtest_runner
cargo build
cargo run -- "$1"
