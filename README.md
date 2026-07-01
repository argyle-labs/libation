<p align="center">
  <img src="assets/icon-256.png" width="120" alt="libation" />
</p>

# libation

Libation downloads and manages your Audible library, converting titles to open formats.

A first-party [orca](https://github.com/argyle-labs/orca) plugin (service-backend).

This repo is **self-contained** — the steps below run libation **by hand, without orca**. orca automates exactly this (same image, ports, and data) through one generic surface.

---

## Run it without orca

### Docker / Podman

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

```sh
docker compose up -d
```

Podman: the same file with `podman-compose up -d`.

### Ports & data

| | |
|---|---|
| Default port | `9494` |
| Upstream | <https://github.com/rmcrackan/Libation> |
| Operator notes | [libation.md](docs/libation.md) |


### Backup & restore

Back up the config/data volume(s) above — that's the whole service state (stop the container first for a clean copy). Restore by putting them back and starting it.

> With orca this is **`service.backup` / `service.restore`** — location-agnostic (docker / podman / lxc / vm), one command regardless of where libation runs. No per-service backup script.

## With orca

orca drives this plugin through the single generic `service.*` surface — no per-plugin tools:

```sh
orca service.deploy libation      # render + launch on any supported runtime
orca service.status libation      # health + rich diagnostics (typed payload)
orca service.backup libation      # location-agnostic backup (tar; PBS on Proxmox)
orca service.configure libation   # apply config via the upstream API
```

## Layout

- `src/` — the plugin (pure Rust): the `ServiceBackend` descriptor + `configure` / `status`.
- `docs/` — standalone operator notes.
- [CAPABILITIES.md](CAPABILITIES.md) — the service-backend contract checklist.
- `assets/` — plugin icon.
