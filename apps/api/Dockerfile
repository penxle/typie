FROM amazonlinux:2023 AS base
ENV NODE_ENV=production

RUN curl -fsSL https://rpm.nodesource.com/setup_24.x | bash
RUN dnf install -y nodejs
RUN corepack enable pnpm && corepack install -g pnpm@latest-10
RUN pnpm config set store-dir /pnpm-store

FROM base AS deps
WORKDIR /deps

COPY out/pnpm-lock.yaml out/pnpm-workspace.yaml ./
COPY patches ./patches
RUN pnpm fetch --prod

COPY out/json ./
RUN pnpm install --offline --prod

FROM base AS runner
WORKDIR /app

ADD https://github.com/krallin/tini/releases/download/v0.19.0/tini-arm64 /tini
RUN chmod +x /tini

ENV LD_LIBRARY_PATH="/opt/sharp/lib"
ADD vendor/sharp.tar.xz /opt/sharp

COPY --from=deps /deps ./
COPY out/full ./

RUN mv /opt/sharp/node_modules /app/apps/api

WORKDIR /app/apps/api
EXPOSE 3000

ENTRYPOINT ["/tini", "--"]
CMD ["pnpm", "start"]
