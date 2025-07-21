FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
	ca-certificates \
	libssl3 \
	&& rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY target/release/magister /usr/local/bin/magister

ENV RUST_LOG=info RUST_LOG_STYLE=always RUST_BACKTRACE=1
CMD ["/usr/local/bin/magister"]
