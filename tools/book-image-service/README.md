# Book Image Service

Lightweight OpenAI-compatible image endpoint for book-summary illustration tests.

Default model:

```text
Lykon/dreamshaper-8-lcm
```

Local endpoint:

```text
POST /v1/images/generations
GET /health
```

MacMini4 deployment:

```text
http://100.96.199.26:30019
```

The service runs from:

```text
/Volumes/System/docker/book-image-service
```

Use `install-watchdog.sh` on MacMini4 to install a cron watchdog. It checks
`/health` every minute and starts `start.sh` when the service is down.

Example:

```bash
curl -X POST http://127.0.0.1:30019/v1/images/generations \
  -H 'Content-Type: application/json' \
  -d '{"prompt":"warm hand-drawn book illustration, stationery shop, camellia, envelope, no text","size":"768x432","n":1}'
```
