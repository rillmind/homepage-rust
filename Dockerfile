# syntax=docker/dockerfile:1

FROM docker.io/oven/bun:1.3 AS front
WORKDIR /app
COPY front-end/package.json front-end/bun.lock ./
RUN bun install --frozen-lockfile
COPY front-end/ ./
# Comprime aqui, para o servidor não gastar CPU com isso.
RUN bun run build && find dist -type f \( -name '*.js' -o -name '*.css' -o -name '*.html' -o -name '*.svg' \) -exec gzip -9 -k {} +

FROM docker.io/library/rust:1.97-alpine AS back
RUN apk add --no-cache musl-dev
WORKDIR /app
# Dependências em uma camada própria: só recompilam quando o Cargo.lock muda.
COPY back-end/Cargo.toml back-end/Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release --locked && rm -r src
COPY back-end/src ./src
RUN touch src/main.rs && cargo build --release --locked

# O binário é estático, então não precisa de um sistema em volta.
FROM scratch
COPY --from=back /app/target/release/homepage-back-end /homepage
COPY --from=front /app/dist /front
ENV LISTEN=5173 DATA_DIR=/data FRONT_DIR=/front
EXPOSE 5173
ENTRYPOINT ["/homepage"]
