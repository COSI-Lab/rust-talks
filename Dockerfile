FROM rust AS builder
WORKDIR /rust-talks

# Build dependencies
RUN echo "fn main() {}" > dummy.rs
COPY Cargo.toml .
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml

# Prepare build
COPY src src
COPY static static
COPY templates templates
ARG VIRTUAL_PORT

# Build release
RUN touch src/main.rs
RUN cargo build --release

# Run binary
FROM rust:slim
WORKDIR /rust_talks
COPY --from=builder /rust_talks/events.txt events.txt
COPY --from=builder /rust_talks/target/release/rust_talks rust_talks
CMD ["./rust_talks"]
