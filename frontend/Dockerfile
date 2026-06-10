# syntax=docker/dockerfile:1.7
FROM node:22-alpine AS builder
WORKDIR /app
RUN corepack enable
COPY package.json pnpm-lock.yaml .npmrc ./
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

FROM node:22-alpine
WORKDIR /app
COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/node_modules ./node_modules
# Non-root, mirroring Dockerfile.frontend (prod). Runtime only reads these
# root-owned, world-readable paths.
USER node
EXPOSE 3000
ENV PORT=3000
CMD ["node", "build"]
