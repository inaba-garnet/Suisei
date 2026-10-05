# syntax=docker/dockerfile:1
# 本番のイメージ（docs/server.md の「Docker イメージ」）。Web を埋め込んだサーバーと ffmpeg を載せる。

FROM node:22-trixie-slim AS web
WORKDIR /src/web
RUN corepack enable
COPY web/package.json web/pnpm-lock.yaml ./
RUN --mount=type=cache,target=/root/.cache/pnpm \
    pnpm fetch --frozen-lockfile
COPY web/ ./
RUN --mount=type=cache,target=/root/.cache/pnpm \
    pnpm install --frozen-lockfile --offline && pnpm build

FROM rust:1-trixie AS server
WORKDIR /src/server
# rust-toolchain.toml の版を入れる
COPY server/rust-toolchain.toml ./
RUN rustup toolchain install
# 依存のクレートと辞書を先に作り、ソースだけを変えたときに作り直さない層にする。
# target はキャッシュマウントにしない。CI の層のキャッシュに残すため
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/dict/ dict/
ENV SUISEI_DICT_CACHE=/root/.cache/suisei-dict
RUN --mount=type=cache,target=/root/.cache/suisei-dict \
    mkdir src && echo 'fn main() {}' > src/main.rs && touch src/lib.rs \
    && cargo build --release --locked && rm -r src
# dict/ は写し直さない。更新時刻が変わると辞書を作り直すため
COPY server/build.rs ./
COPY server/.sqlx/ .sqlx/
COPY server/migrations/ migrations/
COPY server/src/ src/
COPY --from=web /src/web/.output/public /src/web/.output/public
RUN touch src/main.rs src/lib.rs \
    && cargo build --release --locked && cp target/release/suisei /usr/local/bin/suisei

FROM debian:trixie-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ffmpeg \
    && rm -rf /var/lib/apt/lists/*
RUN groupadd --gid 1000 suisei \
    && useradd --uid 1000 --gid 1000 --no-create-home --shell /usr/sbin/nologin suisei \
    && mkdir /data && chown suisei:suisei /data
COPY --from=server /usr/local/bin/suisei /usr/local/bin/suisei
# 配るイメージに、Suisei と依存と辞書のライセンスを同梱する（docs/server.md の「ライセンスの表示」）
COPY LICENSE /usr/share/doc/suisei/
COPY server/THIRD_PARTY_LICENSES.md /usr/share/doc/suisei/THIRD_PARTY_LICENSES-server.md
COPY --from=web /src/web/THIRD_PARTY_LICENSES.md /usr/share/doc/suisei/THIRD_PARTY_LICENSES-web.md
COPY server/dict/README.md /usr/share/doc/suisei/dict/
COPY server/dict/licenses/ /usr/share/doc/suisei/dict/licenses/
USER suisei
ENV SUISEI_DATA_DIR=/data \
    SUISEI_MUSIC_DIR=/music
VOLUME /data
EXPOSE 4533
ENTRYPOINT ["suisei"]
