FROM rust:1.72-slim as BUILDER

WORKDIR /app

COPY . .

RUN cargo build --release

FROM rust:1.72-slim as RELEASE

COPY --from=BUILDER /app/target/release/rinha rinha

ENTRYPOINT [ "./rinha" ]