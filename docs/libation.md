# Libation

Downloads and converts your own Audible purchases to open (DRM-free) formats for
personal use, and manages your Audible library.

- **Ports**: none. The image runs `LibationCli scan` then `LibationCli liberate` every `SLEEP_TIME` (unset or `-1` runs once and exits).
- **Image**: `rmcrackan/libation:latest`
- **Upstream**: <https://github.com/rmcrackan/Libation>

## Volumes

| Container Path | Description |
|----------------|-------------|
| `/config` | Libation config and Audible credentials |
| `/data` | Downloaded audiobooks output |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TZ` | `Etc/UTC` | Timezone |

## Deploy

```yaml
# compose.yml
services:
  libation:
    image: rmcrackan/libation:latest
    container_name: libation
    restart: unless-stopped
    environment:
      - SLEEP_TIME=6h
    volumes:
      - ./config:/config
      - ./books:/data
```

```bash
docker compose up -d
```

## Initial Setup (Audible Authentication)

The container cannot log in interactively. Authenticate once in desktop Libation, then copy
`AccountsSettings.json` and `Settings.json` (and `libation-master.key` if tokens are encrypted)
into `/config`. The entrypoint loads them on start.

## Syncing Your Library

Each loop scans every account and liberates every title not yet liberated into `/data`.
`docker exec libation /libation/LibationCli <verb>` runs one command (`scan`, `liberate`,
`export --json -p <file>`, `search`).

## Troubleshooting

```bash
docker compose logs libation
```

A scan that fails with `Failed to decrypt ExistingAccessToken` needs the master key in `/config`.
