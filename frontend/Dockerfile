# ────────────────────────────────────────────────────────────────────────────────
# SECS Frontend — Multi-stage Dockerfile
# Build: Node.js + pnpm, Production: nginx alpine for static file serving
# ────────────────────────────────────────────────────────────────────────────────

# Stage 1: Build
FROM node:22-alpine AS builder

WORKDIR /app

# Cài pnpm 9 (pnpm 11 block build scripts mặc định, không phù hợp Docker)
RUN corepack enable && corepack prepare pnpm@9 --activate

# Cache dependencies — copy package files trước
COPY package.json pnpm-lock.yaml ./

RUN pnpm install --frozen-lockfile

# Copy source code
COPY index.html ./
COPY postcss.config.js ./
COPY tailwind.config.js ./
COPY vite.config.js ./
COPY src/ src/

# Build production
RUN pnpm build

# Stage 2: Production với nginx
FROM nginx:1.27-alpine AS runtime

# Non-root nginx config
RUN rm /etc/nginx/conf.d/default.conf

# Custom nginx config — SPA routing + reverse proxy cho API
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy static files từ builder
COPY --from=builder /app/dist /usr/share/nginx/html

# Security: remove server version header
RUN sed -i 's/# server_tokens off;/server_tokens off;/' /etc/nginx/nginx.conf || true

EXPOSE 80

HEALTHCHECK --interval=10s --timeout=3s --start-period=2s --retries=3 \
    CMD wget -q --spider http://localhost/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
