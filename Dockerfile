FROM rust:1.97-alpine AS build

RUN apk add --no-cache musl-dev ca-certificates

WORKDIR /app

# cargo deps caching trick
RUN cargo init --bin .
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# copy and build the actual application
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /app/target/release/quickping /server
EXPOSE 3000
ENTRYPOINT ["/server"]
