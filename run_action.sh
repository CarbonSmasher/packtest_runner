#!/bin/sh

echo ::group::Build test runner::
cd /packtest_runner && cargo build
cd /github/workspace
echo ::endgroup::
/packtest_runner/target/debug/packtest_runner \
	--comma-separate "$1" \
	--minecraft-version "$2" \
	--packtest-url "$3" \
	--fabric-api-url "$4" \
	--github
