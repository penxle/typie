FROM amazonlinux:2023 AS base

RUN dnf install -y unzip
RUN curl -fsSL https://bun.sh/install | bash -s "bun-v1.2.13"
ENV PATH="/root/.bun/bin:$PATH"

FROM base AS builder
WORKDIR /build

ARG TURBO_TEAM
ARG TURBO_TOKEN
ENV TURBO_TEAM=${TURBO_TEAM}
ENV TURBO_TOKEN=${TURBO_TOKEN}
ENV TURBO_REMOTE_ONLY=true
ENV NODE_ENV=production

COPY . .
RUN bun install --frozen-lockfile
RUN bun run build

FROM base AS deps
WORKDIR /deps

COPY . .
RUN bun install --production

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
