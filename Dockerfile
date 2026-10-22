# syntax=docker/dockerfile:1

# --- web client -------------------------------------------------------------
FROM node:22-bookworm-slim AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# --- server -----------------------------------------------------------------
FROM rust:1.97-bookworm AS server
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN cargo build --release --locked -p cshape

# --- runtime ----------------------------------------------------------------
FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --home /app cshape \
    && mkdir -p /app/var && chown cshape /app/var
WORKDIR /app
COPY --from=server /src/target/release/cshape /usr/local/bin/cshape
COPY --from=web /web/dist /app/web
COPY data/samples /app/samples
COPY data/complexes /app/complexes
USER cshape
ENV CSHAPE_ADDR=0.0.0.0:8080 \
    CSHAPE_DATA=/app/var \
    CSHAPE_WEB=/app/web \
    CSHAPE_SAMPLES=/app/samples \
    CSHAPE_COMPLEXES=/app/complexes
EXPOSE 8080
VOLUME ["/app/var"]
HEALTHCHECK --interval=30s --timeout=3s CMD ["cshape", "health"]
ENTRYPOINT ["cshape"]
CMD ["serve"]
