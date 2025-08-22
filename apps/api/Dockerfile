FROM amazonlinux:2023 AS base
ENV NODE_ENV=production
ENV PLAYWRIGHT_BROWSERS_PATH=/playwright

# spell-checker:disable-next-line
RUN dnf install -y tar xz unzip ghostscript nspr nss atk at-spi2-atk libxkbcommon libXcomposite libXdamage libXrandr libgbm alsa-lib pango
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:$PATH"

FROM base AS runner
WORKDIR /app

ADD https://github.com/krallin/tini/releases/download/v0.19.0/tini-arm64 /tini
RUN chmod +x /tini

RUN bunx playwright install chromium

COPY . .

RUN bun install --frozen-lockfile --production

RUN tar -xvf vendor/sharp.tar.xz -C /app/apps/api/
ENV LD_LIBRARY_PATH="/app/apps/api/lib"

WORKDIR /app/apps/api
EXPOSE 3000

ENTRYPOINT ["/tini", "--"]
CMD ["bun", "run", "start"]
