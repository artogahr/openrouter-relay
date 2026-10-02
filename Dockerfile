FROM rust:1.98-bookworm AS build

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /app/target/release/apify-openrouter-relay /usr/local/bin/openrouter-relay
CMD ["/usr/local/bin/openrouter-relay"]
