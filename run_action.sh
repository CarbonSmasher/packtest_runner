#!/bin/sh

ls
ls /
cd /packtest_runner && cargo build
/packtest_runner/target/debug/packtest_runner "$1"
