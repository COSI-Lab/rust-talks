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
COPY templates templates
ARG VIRTUAL_PORT

# Build release
RUN touch src/main.rs
RUN cargo build --release

# Run binary
FROM rust:slim
RUN apt-get update && apt-get install -y sqlite3
WORKDIR /app
COPY --from=builder /rust-talks/target/release/rust_talks rust_talks
COPY static static
CMD ["./rust_talks"]