FROM alpine:3.10

WORKDIR /github/workspace

COPY runner /packtest_runner
COPY run_action.sh /packtest_runner.sh

ENTRYPOINT ["/packtest_runner.sh"]
