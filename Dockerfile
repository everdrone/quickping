# syntax=docker/dockerfile:1
FROM rust:1.97-alpine AS build

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && cp target/release/quickping /quickping

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /quickping /server
EXPOSE 3000
ENTRYPOINT ["/server"]
