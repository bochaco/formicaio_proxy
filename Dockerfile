FROM rust:1-alpine AS builder
WORKDIR /
COPY . .

RUN apk add pkgconfig openssl-dev libc-dev

RUN cargo build --release
RUN ls -la /target/release

FROM alpine AS runtime
WORKDIR /app

COPY --from=builder /target/release/formicaio_proxy /app/
RUN ls -la /app/

ENV FORMICAIO_ADDR="127.0.0.1:3000"
ENV FORMICAIO_PROXY_PORT=52100

EXPOSE 52100

CMD ["/app/formicaio_proxy"]
