# QuickPing

Simple terminal service to notify yourself when something happens.

```sh
curl localhost:3000 -d 'it has happened'
```

## Docker Compose

```yaml
services:
  quickping:
    image: ghcr.io/everdrone/quickping:latest
    ports:
      - "3000:3000"
    configs:
      - source: qp_config
        target: /etc/quickping/config.toml
    restart: unless-stopped

configs:
  qp_config:
    content: |
      [default]
      url = "https://discord.com/api/webhooks/..."

      [grafana]
      id = 1234567890...
      token = "aBcDEfGhiJkl..."
```
