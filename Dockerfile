##############################################
#  Dockerfile for an x86_64 linux agent image.
##############################################

FROM alpine:3.15

MAINTAINER Cluvio <hi@cluvio.com>

RUN addgroup -S cluvio && adduser -S cluvio -G cluvio

USER cluvio

COPY --chown=cluvio build/cluvio-agent /opt/cluvio/cluvio-agent
COPY --chown=cluvio docker/run-agent.sh /opt/cluvio/run-agent.sh

WORKDIR /opt/cluvio
ENTRYPOINT ["/opt/cluvio/run-agent.sh"]
