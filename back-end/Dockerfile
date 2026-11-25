FROM docker.io/library/rust:1.97-slim-trixie AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM docker.io/library/debian:trixie-slim
COPY --from=build /app/target/release/homepage-back-end /usr/local/bin/
USER nobody
ENV DATA_PATH=/data/services.json
EXPOSE 2306
CMD ["homepage-back-end"]
