# Libation

Audible audiobook downloader and DRM remover. Downloads your Audible purchases as DRM-free m4b files.

- **Host**: <host> (<ip>)
- **Port**: 8008 (configurable via `LIBATION_PORT`)
- **Image**: `ghcr.io/rmcrackan/libation`
- **Compose**: [compose/libation/docker-compose.yml](../../compose/libation/docker-compose.yml)

## Volumes

| Host Path | Container Path | Description |
|-----------|---------------|-------------|
| `/opt/appdata/libation` | `/config` | Libation config and Audible credentials |
| `/mnt/<host>/data/media/audiobooks/audible` | `/data` | Downloaded audiobooks output |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TZ` | `Etc/UTC` | Timezone |
| `LIBATION_IMAGE_TAG` | `latest` | Image tag |
| `LIBATION_CONFIG_PATH` | `/opt/appdata/libation` | Config directory |
| `LIBATION_PORT` | `8008` | Host port |
| `MEDIA_PATH` | `/mnt/<host>/data/media` | Media base path |

## Deploy

```bash
cd compose/libation
docker compose up -d
```

## Initial Setup (Audible Authentication)

Libation requires a one-time browser-based Audible login:

1. Open `http://<ip>:8008`
2. Go to **Settings → Account → Add Account**
3. Select your Audible locale (US = audible.com)
4. Click **Authenticate** — this opens an Audible login URL
5. Complete login in your browser
6. Libation captures the token and saves credentials to `/config`

Authentication only needs to be done once. Credentials persist in the config volume.

## Syncing Your Library

After authentication:

1. **Library → Scan Library** — pulls your full Audible purchase list
2. **Library → Download All** — downloads and decrypts all purchases to `/data` as `.m4b`
3. Files land at `/mnt/<host>/data/media/audiobooks/audible/` on <host>

Audiobookshelf will pick them up automatically on its next scan.

## Ongoing Sync

Libation can be configured to auto-scan on a schedule:

- Settings → Scheduled Tasks → enable **Auto-sync library**
- Recommend: daily sync to catch new purchases

## Troubleshooting

```bash
docker compose logs libation
```

First start pulls the image — may take a minute before the web UI is available.
