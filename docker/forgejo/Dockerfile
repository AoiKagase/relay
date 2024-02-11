FROM alpine:3.19

ARG UID=991
ARG GID=991

ENV \
  UID=${UID} \
  GID=${GID}

USER root
RUN \
  addgroup -g "${GID}" app && \
  adduser -D -G app -u "${UID}" -g "" -h /opt/app app && \
  apk add tini && \
  chown -R app:app /mnt

COPY relay /usr/local/bin/relay

USER app
EXPOSE 6669
EXPOSE 8080
VOLUME /mnt
ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/usr/local/bin/relay"]
