FROM alpine:3.10

WORKDIR /github/workspace

COPY runner /github/workspace/packtest_runner
COPY run_action.sh /github/workspace/packtest_runner.sh

ENTRYPOINT ["/packtest_runner.sh"]
