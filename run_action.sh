#!/bin/sh

echo ::group::Build test runner::
[[ $1 == build ]] && cd /packtest_runner && cargo build
cd /github/workspace
echo ::endgroup::
/packtest_runner/target/debug/packtest_runner \
	--comma-separate "$2" \
	--minecraft-version "$3" \
	--packtest-url "$4" \
	--fabric-api-url "$5" \
	--github
