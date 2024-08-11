FROM rust AS builder

WORKDIR /wingman
COPY . /wingman
RUN RUSTUP_PERMIT_COPY_RENAME=1 cargo build --locked --profile=production

FROM docker.io/library/ubuntu:24.04

COPY --from=builder /wingman/target/production/wingman /usr/local/bin

ENV RUST_LOG=info
ENV AW_PORT=8000
EXPOSE 8000

ENTRYPOINT ["/usr/local/bin/wingman"]
