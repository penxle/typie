FROM oven/bun:1.2 AS build-base
WORKDIR /app
ENV NODE_ENV=production

FROM build-base AS build-deps

COPY out/json/ .
COPY out/full/packages/sark/bin/sark.js ./packages/sark/bin/sark.js
RUN bun install --no-save

FROM build-deps AS build

COPY out/full/ .
RUN bunx --bun turbo build --filter=@typie/website
RUN find . -type d -name "node_modules" -prune -exec rm -rf {} +

FROM debian:12-slim AS runtime-base
WORKDIR /app

ENV PATH="/root/.bun/bin:$PATH"
ENV NODE_ENV=production

RUN apt-get update && apt-get install -y --no-install-recommends \
    tini \
    ca-certificates \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://bun.sh/install | bash
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

FROM runtime-base AS runtime-deps

COPY out/json/ .
RUN bun install --no-save --omit=dev

FROM runtime-deps AS runtime

COPY --from=build /app/ .

ENTRYPOINT ["tini", "--"]
CMD ["bun", "run", "--cwd", "apps/website", "start"]
