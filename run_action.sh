#!/bin/sh

ls
ls /
cd /packtest_runner && cargo build
cd /github/workspace
/packtest_runner/target/debug/packtest_runner --comma-separate "$1"
