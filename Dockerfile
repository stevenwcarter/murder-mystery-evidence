FROM docker.io/blackdex/rust-musl:x86_64-musl AS builder
WORKDIR /home/rust/src

# Dependency cache layer: the crate has a bin and a lib target, so stub both.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && : > src/lib.rs \
    && cargo build --release \
    && rm -r src

# Real build: rust-embed reads Evidence/ and assets/ relative to the manifest dir.
COPY src ./src/
COPY assets ./assets/
COPY Evidence ./Evidence/
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM scratch
COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/murder-mystery-evidence /murder-mystery-evidence
USER 65532:65532
ENV MM_LISTEN=0.0.0.0:8080
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD ["/murder-mystery-evidence", "healthcheck"]
ENTRYPOINT ["/murder-mystery-evidence"]
