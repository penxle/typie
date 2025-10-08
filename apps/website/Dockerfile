FROM oven/bun:1.2 AS source
WORKDIR /app

COPY apps/ ./apps/
COPY packages/ ./packages/
COPY crates/ ./crates/
COPY package.json bun.lock turbo.json ./

RUN bunx --bun turbo prune @typie/website --docker

FROM oven/bun:1.2 AS build-deps
WORKDIR /app

COPY --from=source /app/out/json/ .
COPY --from=source /app/out/full/packages/sark/bin/sark.js ./packages/sark/bin/sark.js
RUN bun install --no-save

FROM build-deps AS build
WORKDIR /app

ENV NODE_ENV=production

COPY --from=source /app/out/full/ .
RUN bunx --bun turbo build --filter=@typie/website
RUN find . -type d -name "node_modules" -prune -exec rm -rf {} +

FROM debian:12-slim AS runtime-deps
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

COPY --from=source /app/out/json/ .
RUN bun install --no-save --omit=dev

FROM runtime-deps AS runtime
WORKDIR /app

COPY --from=build /app/ .

WORKDIR /app/apps/website
ENTRYPOINT ["tini", "--"]
CMD ["bun", "run", "start"]
