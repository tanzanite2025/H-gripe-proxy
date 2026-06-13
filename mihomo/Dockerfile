FROM alpine:latest AS builder
ARG TARGETPLATFORM
RUN echo "I'm building for $TARGETPLATFORM"

RUN apk add --no-cache gzip && \
    mkdir /mihomo-config
COPY docker/geodata/ /mihomo-config/
RUN test -s /mihomo-config/geoip.metadb || (echo "missing docker/geodata/geoip.metadb; provide controlled local geodata before building" >&2; exit 1) && \
    test -s /mihomo-config/geosite.dat || (echo "missing docker/geodata/geosite.dat; provide controlled local geodata before building" >&2; exit 1) && \
    test -s /mihomo-config/geoip.dat || (echo "missing docker/geodata/geoip.dat; provide controlled local geodata before building" >&2; exit 1)

COPY docker/file-name.sh /mihomo/file-name.sh
WORKDIR /mihomo
COPY bin/ bin/
RUN FILE_NAME=`sh file-name.sh` && echo $FILE_NAME && \
    FILE_NAME=`ls bin/ | egrep "$FILE_NAME.gz"|awk NR==1` && echo $FILE_NAME && \
    mv bin/$FILE_NAME mihomo.gz && gzip -d mihomo.gz && chmod +x mihomo && echo "$FILE_NAME" > /mihomo-config/test
FROM alpine:latest
LABEL org.opencontainers.image.source="https://github.com/tanzanite2025/clash-verge-optimized"

RUN apk add --no-cache ca-certificates tzdata iptables

VOLUME ["/root/.config/mihomo/"]

COPY --from=builder /mihomo-config/ /root/.config/mihomo/
COPY --from=builder /mihomo/mihomo /mihomo
ENTRYPOINT [ "/mihomo" ]
