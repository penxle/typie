FROM debian:12-slim AS builder
WORKDIR /app

ENV PATH="/root/.bun/bin:$PATH"
ENV NODE_ENV=production

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    python3 \
    tar \
    unzip \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://bun.sh/install | bash
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

FROM builder AS build

COPY out/json/ .
RUN bun install --no-save

COPY out/full/ .
RUN bunx turbo build --filter=@typie/api
RUN find . -type d -name "node_modules" -prune -exec rm -rf {} +

FROM builder AS prod-deps

COPY out/json/ .
RUN bun install --no-save --omit=dev

FROM builder AS sharp

COPY vendor/sharp-debian.tar.xz ./vendor/
RUN tar -xf vendor/sharp-debian.tar.xz -C apps/api/ && rm -rf vendor

FROM debian:12-slim AS runtime
WORKDIR /app

ENV PATH="/root/.bun/bin:$PATH"
ENV LD_LIBRARY_PATH="/app/apps/api/lib"
ENV NODE_ENV=production

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    ffmpeg \
    ghostscript \
    tini \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /root/.bun /root/.bun

RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

COPY --from=prod-deps /app/ .
RUN cd apps/api && npx playwright install-deps && npx playwright install chromium

COPY --from=build /app/ .
COPY --from=sharp /app/apps/api/lib/ ./apps/api/lib/

ENTRYPOINT ["tini", "--"]
CMD ["bun", "run", "--cwd", "apps/api", "start"]
