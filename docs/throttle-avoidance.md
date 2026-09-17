# Avoiding Audible account throttling (operational KB)

Complements [libation.md](libation.md). The single most important operational rule
for Libation: **do not run `liberate` in a tight loop.**

## The 6-hour liberate-all loop throttles the whole account

Running a full `liberate` on a short cadence (e.g. `SLEEP_TIME=6h` in an infinite
loop) re-requests an Audible content license for every owned-but-undownloaded
title on **every** cycle. Audible responds by throttling the **entire account**
("your account is being throttled, wait 24–48 hours"), which denies *all* license
grants — including new titles you actually want. Retrying during the window can
re-extend it.

**Fix:** run once / diff-only. Set `SLEEP_TIME=-1` so the container performs a
single scan+liberate cycle and exits rather than looping. Trigger liberation
manually or on a throttle-safe schedule (daily/weekly), never in a tight loop.

**Recreate the container to bake it in.** A *stopped* container keeps its old
environment — editing the compose file is not enough. Recreate it from the fixed
compose so the new `SLEEP_TIME` actually takes effect, e.g.:

```
docker compose up -d --no-start   # recreate with new env, leave stopped
docker start libation             # fires exactly ONE scan+liberate, then exits
```

## Liberating a single title without a full scan

`liberate` only fetches titles not yet liberated. To fetch one specific title
without the scan+loop, pass the ASIN as an argument — the entrypoint execs passed
arguments *before* the liberate loop, so no scan runs:

```
LibationCli liberate --limit-books 1 <ASIN>
```

(A title must not be "absent from the last library scan" for the single-title path
to work without a scan; otherwise a scan — which is itself a throttle trigger — is
needed to refresh library state.)

## The throttle is ACCOUNT-level, not VPN/IP — do not chase network fixes

Measured: the host running Libation egressed a clean residential IP (no VPN
sidecar) and was **still** throttled. The throttle is Audible rate-limiting the
account based on request velocity, not a poisoned shared VPN exit. Do **not** move
hosts, build an audible.com VPN bypass, or relocate to another machine on the same
IP — none of these address an account-level throttle. The only real remedy is a
cooldown followed by a single, scoped `liberate`. If it still throttles from a
clean residential IP after cooldown, it is pure account velocity — space requests
further apart.
