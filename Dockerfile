FROM amazonlinux:2023 AS base

RUN curl -fsSL https://rpm.nodesource.com/setup_24.x | bash
RUN dnf install -y nodejs
RUN corepack enable

FROM base AS builder
WORKDIR /build

ARG TURBO_TEAM
ARG TURBO_TOKEN
ENV TURBO_TEAM=${TURBO_TEAM}
ENV TURBO_TOKEN=${TURBO_TOKEN}
ENV TURBO_REMOTE_ONLY=true
ENV NODE_ENV=production

COPY . .
RUN pnpm install --frozen-lockfile
RUN pnpm run build

FROM base AS deps
WORKDIR /deps

COPY . .
RUN pnpm install --frozen-lockfile --production

FROM base AS runner
WORKDIR /app

ADD https://github.com/krallin/tini/releases/download/v0.19.0/tini-arm64 /tini
RUN chmod +x /tini

ENV NODE_ENV=production

ENV LD_LIBRARY_PATH="/app/lib"
ADD vendor/sharp.tar.xz .

COPY --from=builder /build/apps/api/dist ./apps/api
COPY --from=builder /build/apps/website/dist ./apps/website

COPY --from=deps /deps/node_modules ./node_modules

EXPOSE 3000
ENTRYPOINT ["/tini", "--"]
