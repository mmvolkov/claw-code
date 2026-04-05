FROM rust:1-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    ca-certificates \
    curl \
    git \
    python3 \
    ripgrep \
 && rm -rf /var/lib/apt/lists/*

RUN rustup component add clippy rustfmt

WORKDIR /opt/claw-code

COPY rust ./rust

WORKDIR /opt/claw-code/rust
RUN cargo build -p rusty-claude-cli -p web-api
RUN install -m 0755 target/debug/claw /usr/local/bin/claw
RUN install -m 0755 target/debug/claw-web /usr/local/bin/claw-web

WORKDIR /workspace

EXPOSE 8787

ENTRYPOINT ["claw"]
CMD ["--help"]
