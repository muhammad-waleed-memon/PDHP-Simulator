# ============================================================
# Frontend — Multi-Stage React Build
# ============================================================
# Stage 1: Build React app with Vite
# Stage 2: Serve static files with Nginx
# ============================================================

# ── Stage 1: Build ────────────────────────────────────────────
FROM node:20-alpine AS builder

WORKDIR /app

# Dependency caching
COPY package.json package-lock.json* ./
RUN npm ci --ignore-scripts

# Build arguments for environment variables baked into the build
ARG VITE_API_BASE_URL=/api/v1
ARG VITE_FHIR_BASE_URL=/fhir
ENV VITE_API_BASE_URL=$VITE_API_BASE_URL
ENV VITE_FHIR_BASE_URL=$VITE_FHIR_BASE_URL

# Copy source and build
COPY . .
RUN npm run build

# ── Stage 2: Serve ────────────────────────────────────────────
FROM nginx:1.25-alpine AS runtime

# Remove default nginx config
RUN rm /etc/nginx/conf.d/default.conf

# SPA-friendly Nginx config
COPY --from=builder /app/dist /usr/share/nginx/html

# Nginx config for SPA (React Router support)
RUN cat > /etc/nginx/conf.d/default.conf << 'EOF'
server {
    listen 80;
    server_name localhost;
    root /usr/share/nginx/html;
    index index.html;

    # Gzip
    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml text/javascript;

    # Static assets with caching
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
        try_files $uri =404;
    }

    # SPA fallback — all unknown routes served by index.html
    location / {
        try_files $uri $uri/ /index.html;
    }
}
EOF

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -q --spider http://127.0.0.1/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
