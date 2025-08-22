FROM amazonlinux:2023 AS base
ENV NODE_ENV=production

RUN dnf install -y unzip
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:$PATH"

FROM base AS runner
WORKDIR /app

ADD https://github.com/krallin/tini/releases/download/v0.19.0/tini-arm64 /tini
RUN chmod +x /tini

COPY . .

RUN bun install --frozen-lockfile --production

WORKDIR /app/apps/website
EXPOSE 3000

ENTRYPOINT ["/tini", "--"]
CMD ["bun", "run", "start"]
