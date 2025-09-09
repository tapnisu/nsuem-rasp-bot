FROM rust:alpine3.22 AS builder
LABEL authors="tapnisu"

WORKDIR /usr/src/nsuem-rasp-bot

RUN apk update \
    && apk upgrade --available \
    && apk add --no-cache alpine-sdk libressl-dev

COPY . .
RUN cargo build --release

FROM alpine:3.22 AS runner

RUN apk update \
    && apk upgrade --available \
    && apk add --no-cache ca-certificates \
    && update-ca-certificates

COPY --from=builder /usr/src/nsuem-rasp-bot/target/release/nsuem-rasp-bot /usr/local/bin/nsuem-rasp-bot

CMD ["nsuem-rasp-bot"]