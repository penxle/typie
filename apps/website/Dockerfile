FROM node:25-slim AS builder
WORKDIR /app

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

RUN npm install -g pnpm

FROM builder AS build

ARG IMAGE_TAG
ENV PUBLIC_IMAGE_TAG=$IMAGE_TAG

COPY out/json/ .
RUN pnpm install --frozen-lockfile

COPY out/full/ .
RUN pnpm exec turbo build --filter=@typie/website
RUN find . -type d -name "node_modules" -prune -exec rm -rf {} +

FROM builder AS prod-deps

COPY out/json/ .
RUN pnpm install --frozen-lockfile --prod

FROM node:25-slim AS runtime
WORKDIR /app

ARG IMAGE_TAG
ENV PUBLIC_IMAGE_TAG=$IMAGE_TAG
ENV NODE_ENV=production

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tini \
    && rm -rf /var/lib/apt/lists/*

COPY --from=prod-deps /app/ .
COPY --from=build /app/ .

ENTRYPOINT ["tini", "--"]
CMD ["node", "apps/website/dist/index.js"]
