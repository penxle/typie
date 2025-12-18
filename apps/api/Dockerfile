FROM debian:12-slim AS base
WORKDIR /app

ENV PATH="/root/.bun/bin:$PATH"
ENV LD_LIBRARY_PATH="/app/apps/api/lib"
ENV NODE_ENV=production

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    ghostscript \
    git \
    python3 \
    tar \
    tini \
    unzip \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://bun.sh/install | bash
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

FROM base AS build-deps

COPY out/json/ .
RUN bun install --no-save

FROM build-deps AS build

COPY out/full/ .
RUN bunx turbo build --filter=@typie/api
RUN find . -type d -name "node_modules" -prune -exec rm -rf {} +

FROM base AS runtime-deps

COPY out/json/ .
RUN bun install --no-save --omit=dev
RUN cd apps/api && npx playwright install-deps && npx playwright install chromium

FROM runtime-deps AS runtime

COPY --from=build /app/ .
COPY vendor/sharp-debian.tar.xz ./vendor/

RUN tar -xf vendor/sharp-debian.tar.xz -C apps/api/ && rm -rf vendor

ENTRYPOINT ["tini", "--"]
CMD ["bun", "run", "--cwd", "apps/api", "start"]
