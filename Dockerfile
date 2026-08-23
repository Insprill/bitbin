####################################################################################################
## Builder
####################################################################################################
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /bitbin

COPY . .

# Set environment variables so the build has git info
RUN export $(cat .env | xargs) && cargo build --release

####################################################################################################
## Final image
####################################################################################################
FROM alpine:latest

WORKDIR /opt/bitbin
COPY --from=builder /bitbin/target/release/bitbin .

RUN mkdir content db
VOLUME ["/opt/bitbin/content", "/opt/bitbin/db"]

HEALTHCHECK --interval=1m --timeout=5s CMD wget --spider -q http://localhost:8080/health || exit 1

EXPOSE 8080/tcp
CMD ["./bitbin"]
