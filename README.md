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
        mode: 0444
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

## Usage

To quickly send a message

```sh
curl localhost:3000 -d 'ping'
```

To send it to a specific profile

```sh
curl localhost:3000/grafana -d 'disk almost full'
```

Messages can be customized using discord's event [webhook content schema](https://docs.discord.com/developers/resources/webhook#execute-webhook)

```sh
curl localhost:3000 -d '{
    "embeds": [
        { "title": "embed title", "color": 5763719 }
    ],
    "username": "CustomName"
}'
```

## Configuration

Each `[profile]` is a webhook, given as `url` or as `id` + `token`. A profile without credentials posts to the `[default]` webhook.

Message fields can be set directly in the configuration file: `username`, `avatar_url`, `tts`, `allowed_mentions`, `flags`, `thread_name` and `applied_tags`. Fields sent in the request win over the profile, and the profile wins over `[default]`.

```toml
[default]
url = "https://discord.com/api/webhooks/..."
username = "QuickPing"

[grafana]
# same webhook as [default], different name and avatar
username = "Grafana"
avatar_url = "https://example.com/grafana.png"

[backups]
id = 1234567890
token = "aBcDEfGhiJkl..."
flags = 4096 # silent
```

The config path can be changed with the `QP_CONFIG` environment variable (default: `/etc/quickping/config.toml`).
