# libation — ServiceBackend contract checklist

Driven by the generic `service.*` surface (no per-plugin tools). `[ ]` =
scaffolded stub. Modalities: **docker,podman,lxc**.

## ServiceBackend methods
- [ ] `provider` / `modalities` / `default_port` (declared)
- [ ] `deploy(modality)` — docker/podman/lxc/vm as applicable
- [ ] `backup`
- [ ] `restore`
- [ ] `configure` — service-specific config
- [ ] `status` — health/diagnostics
- [ ] connect/sync handled generically by the toolkit (endpoint registry + peer sync)

## Deploy modalities
- [ ] docker compose
- [ ] podman compose
- [ ] LXC (pct)
- n/a vm
- n/a device
- n/a host
