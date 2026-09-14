FROM rust:1-slim-trixie AS builder

WORKDIR /app

ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
    CARGO_PROFILE_RELEASE_LTO=true \
    CARGO_PROFILE_RELEASE_PANIC=abort \
    CARGO_NET_RETRY=10

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

COPY src ./src
RUN find src -type f -exec touch {} + \
    && cargo build --release --locked \
    && cp target/release/simple-opengraph-scraper /tmp/bin


FROM gcr.io/distroless/cc-debian13

USER nonroot:nonroot

EXPOSE 3000

COPY --from=builder /tmp/bin /simple-opengraph-scraper

ENTRYPOINT ["/simple-opengraph-scraper"]
