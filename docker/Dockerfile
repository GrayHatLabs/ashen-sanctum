# The Ashen Sanctum login server (src/server). Build from the repository's top folder:
#   docker compose -f docker/docker-compose.yml up -d --build
# See docker/README.md.

# ---- build ----
FROM rust:1-bookworm AS build
RUN apt-get update \
 && apt-get install -y --no-install-recommends libsdl2-dev \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs ./
COPY src src
COPY assets assets
COPY levels levels
RUN cargo build --release --locked

# ---- run ----
FROM debian:bookworm-slim
# The game's own library (the server runs with no window or sound), and nothing else.
RUN apt-get update \
 && apt-get install -y --no-install-recommends libsdl2-2.0-0 \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin ashen \
 && mkdir /data && chown 10001:10001 /data && chmod 700 /data
COPY --from=build /src/target/release/ashensanctum /usr/local/bin/ashensanctum
USER 10001:10001
ENV ASHEN_DATA=/data \
    SDL_VIDEODRIVER=dummy \
    SDL_AUDIODRIVER=dummy
VOLUME /data
EXPOSE 4747/tcp
ENTRYPOINT ["/usr/local/bin/ashensanctum", "--server"]
