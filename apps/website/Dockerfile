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

ARG IMAGE_TAG
ENV PUBLIC_IMAGE_TAG=$IMAGE_TAG

COPY out/json/ .
RUN bun install --no-save

COPY out/full/ .
RUN bunx turbo build --filter=@typie/website
RUN find . -type d -name "node_modules" -prune -exec rm -rf {} +

FROM builder AS prod-deps

COPY out/json/ .
RUN bun install --no-save --omit=dev

FROM debian:12-slim AS runtime
WORKDIR /app

ARG IMAGE_TAG
ENV PUBLIC_IMAGE_TAG=$IMAGE_TAG
ENV PATH="/root/.bun/bin:$PATH"
ENV NODE_ENV=production

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tini \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /root/.bun /root/.bun

COPY --from=prod-deps /app/ .
COPY --from=build /app/ .

ENTRYPOINT ["tini", "--"]
CMD ["bun", "run", "--cwd", "apps/website", "start"]
