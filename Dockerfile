FROM alpine:3.10

COPY target/debug/packtest_runner /run

ENTRYPOINT ["/run"]
