# TODO: base image + build for libation. Mirror jellyfin/Dockerfile conventions.
FROM debian:12-slim
LABEL org.opencontainers.image.source="https://github.com/argyle-labs/libation"
EXPOSE 9494
