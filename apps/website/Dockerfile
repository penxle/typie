FROM debian:12-slim AS base
ENV NODE_ENV=production

RUN apt-get update && apt-get install -y \
    tini \
    curl \
    unzip \
    && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:$PATH"

FROM base AS runner
WORKDIR /app

COPY . .

RUN bun install --frozen-lockfile --production

WORKDIR /app/apps/website
EXPOSE 3000

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["bun", "run", "start"]
