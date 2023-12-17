FROM rust:1.73.0

# WORKDIR /github/workspace

ADD runner /packtest_runner
COPY run_action.sh /packtest_runner.sh

ENTRYPOINT ["/packtest_runner.sh"]
