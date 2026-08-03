FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY . .
RUN cargo build --release --bin qrender-server

FROM alpine:3
RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/qrender-server /usr/local/bin/qrender-server
EXPOSE 4243
CMD ["qrender-server"]
