# Libation

Downloads and converts your own Audible purchases to open (DRM-free) formats for
personal use, and manages your Audible library.

- **Port**: `9494` (web UI)
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
    ports:
      - "9494:9494/tcp"   # web UI
    volumes:
      - ./config:/config
      - ./books:/data
```

```bash
docker compose up -d
```

## Initial Setup (Audible Authentication)

Libation requires a one-time browser-based Audible login:

1. Open `http://<host>:9494`
2. Go to **Settings → Account → Add Account**
3. Select your Audible locale (US = audible.com)
4. Click **Authenticate** — this opens an Audible login URL
5. Complete login in your browser
6. Libation captures the token and saves credentials to `/config`

Authentication only needs to be done once. Credentials persist in the config volume.

## Syncing Your Library

After authentication:

1. **Library → Scan Library** — pulls your full Audible purchase list
2. **Library → Download All** — downloads and converts your purchases to `/data` as `.m4b`

## Ongoing Sync

Libation can be configured to auto-scan on a schedule:

- Settings → Scheduled Tasks → enable **Auto-sync library**
- Recommend: daily sync to catch new purchases

## Troubleshooting

```bash
docker compose logs libation
```

First start pulls the image — may take a minute before the web UI is available.
