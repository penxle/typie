FROM oven/bun:1 AS base

FROM base AS builder
WORKDIR /build

ARG TURBO_TEAM
ARG TURBO_TOKEN
ENV TURBO_TEAM=${TURBO_TEAM}
ENV TURBO_TOKEN=${TURBO_TOKEN}
ENV TURBO_REMOTE_ONLY=true
ENV NODE_ENV=production

COPY . .
RUN bun install --frozen-lockfile --ignore-scripts
RUN bun run build

FROM base AS deps
WORKDIR /deps

COPY . .
RUN bun install --production

FROM base AS runner
WORKDIR /app

ENV NODE_ENV=production

COPY --from=builder /build/apps/api/dist ./apps/api

COPY --from=deps /deps/node_modules ./node_modules
