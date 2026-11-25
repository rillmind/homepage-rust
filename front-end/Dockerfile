FROM docker.io/oven/bun:1.3 AS build
WORKDIR /app
COPY package.json bun.lock ./
RUN bun install --frozen-lockfile
COPY . .
RUN bun run build

FROM docker.io/library/nginx:stable-alpine
COPY --from=build /app/dist /usr/share/nginx/html
ENV LISTEN=5173 BACKEND=127.0.0.1:2306
COPY nginx.conf /etc/nginx/templates/default.conf.template
EXPOSE 5173
