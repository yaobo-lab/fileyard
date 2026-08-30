# syntax=docker/dockerfile:1

# ─── Dependencies ─────────────────────────────────────────────────────────────
# Install with the lockfile only, so this layer is cached until deps change.
FROM oven/bun:1.3.14 AS dependencies
WORKDIR /app

COPY package.json bun.lock ./
RUN bun install --frozen-lockfile

# ─── Build ────────────────────────────────────────────────────────────────────
FROM oven/bun:1.3.14 AS builder
WORKDIR /app

# NEXT_PUBLIC_* values are inlined into the client bundle at build time, so they
# must be provided here (not at runtime). Override with --build-arg as needed.
ARG NEXT_PUBLIC_APP_URL=http://localhost:3000
ARG NEXT_PUBLIC_BASE_PATH=
ARG NEXT_PUBLIC_ASSET_PREFIX=
ENV NEXT_PUBLIC_APP_URL=$NEXT_PUBLIC_APP_URL \
    NEXT_PUBLIC_BASE_PATH=$NEXT_PUBLIC_BASE_PATH \
    NEXT_PUBLIC_ASSET_PREFIX=$NEXT_PUBLIC_ASSET_PREFIX \
    NEXT_TELEMETRY_DISABLED=1

COPY --from=dependencies /app/node_modules ./node_modules
COPY . .
RUN bun run build

# ─── Runtime ──────────────────────────────────────────────────────────────────
FROM oven/bun:1.3.14 AS runner
WORKDIR /app

ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3000 \
    HOSTNAME=0.0.0.0

# Ship only what `next start` needs to run: build output, static assets,
# config, and the installed dependencies.
COPY --from=builder --chown=bun:bun /app/.next ./.next
COPY --from=builder --chown=bun:bun /app/public ./public
COPY --from=builder --chown=bun:bun /app/node_modules ./node_modules
COPY --from=builder --chown=bun:bun /app/package.json ./package.json
COPY --from=builder --chown=bun:bun /app/next.config.mjs ./next.config.mjs

# Server-side connection and auth vars (STORAGE_*, AUTH_*) are read at runtime;
# pass them with `-e` / `--env-file` when starting the container.

USER bun
EXPOSE 3000
CMD ["bun", "run", "start"]
