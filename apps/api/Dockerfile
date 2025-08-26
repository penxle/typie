FROM debian:12-slim AS base
ENV NODE_ENV=production

RUN apt-get update && apt-get install -y \
    build-essential \
    tini \
    curl \
    ca-certificates \
    gnupg \
    unzip \
    tar \
    xz-utils \
    ghostscript \
    && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash -
RUN curl -fsSL https://bun.sh/install | bash
RUN apt-get install -y nodejs
ENV PATH="/root/.bun/bin:$PATH"

FROM base AS runner
WORKDIR /app

RUN npx playwright install-deps
RUN npx playwright install chromium

COPY . .

RUN bun install --frozen-lockfile --production

RUN tar -xvf vendor/sharp-debian.tar.xz -C /app/apps/api/
ENV LD_LIBRARY_PATH="/app/apps/api/lib"

WORKDIR /app/apps/api
EXPOSE 3000

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["bun", "run", "start"]
