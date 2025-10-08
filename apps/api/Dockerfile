FROM oven/bun:1.2 AS source
WORKDIR /app

COPY apps/ ./apps/
COPY packages/ ./packages/
COPY crates/ ./crates/
COPY vendor/ ./vendor/
COPY package.json bun.lock turbo.json ./

RUN bunx turbo prune @typie/api --docker

FROM oven/bun:1.2 AS build-deps
WORKDIR /app

COPY --from=source /app/out/json/ .
RUN bun install --no-save

FROM build-deps AS build
WORKDIR /app

ENV NODE_ENV=production

COPY --from=source /app/out/full/ .
RUN bunx turbo build --filter=@typie/api
RUN find . -type d -name "node_modules" -prune -exec rm -rf {} +

FROM debian:12-slim AS runtime-deps
WORKDIR /app

ENV PATH="/root/.bun/bin:$PATH"
ENV LD_LIBRARY_PATH="/app/apps/api/lib"
ENV NODE_ENV=production

RUN apt-get update && apt-get install -y --no-install-recommends \
    tini \
    ca-certificates \
    curl \
    unzip \
    tar \
    xz-utils \
    ghostscript \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://bun.sh/install | bash

COPY --from=source /app/out/json/ .
RUN bun install --no-save --omit=dev
RUN cd apps/api && bunx playwright install-deps && bunx playwright install chromium

FROM runtime-deps AS runtime
WORKDIR /app

COPY --from=build /app/ .
COPY --from=source /app/vendor/sharp-debian.tar.xz ./vendor/

RUN tar -xf vendor/sharp-debian.tar.xz -C apps/api/ && rm -rf vendor

WORKDIR /app/apps/api
ENTRYPOINT ["tini", "--"]
CMD ["bun", "run", "start"]
