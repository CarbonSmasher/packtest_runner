#!/bin/sh

echo ::group::Build test runner::
cd /packtest_runner && cargo build
cd /github/workspace
echo ::endgroup::
/packtest_runner/target/debug/packtest_runner --comma-separate --github --version "$2" "$1"
