//! libation service backend — Audible library downloader.
//!
//! Implements `ServiceBackend` so the generic `service.*` tools
//! (deploy/backup/restore/configure/status/connect/sync) drive libation. No
//! `#[orca_tool]`s — the only orca dep is `plugin-toolkit`. Modeled on the
//! nfs StorageBackend. See orca/docs/PLUGIN-PROGRAM.md.
#![allow(clippy::disallowed_types)]

use plugin_toolkit::service::{
    BoxFuture, EnvVar, Mount, Routes, Runtime, ServiceBackend, ServiceCapability, ServiceError,
    ServiceStatus, WorkloadSpec,
};

/// Seconds between liberate passes. The entrypoint passes this straight to
/// `sleep(1)`, so it is numeric — `6h` relies on suffix support the image does
/// not guarantee.
///
/// NEVER ship `-1` here. `-1` means "run once then exit", which under a
/// respawning restart policy became a ~63s scan+liberate loop against Audible
/// (~1437 cycles in 25h, 13 days running) and earned an account-level licensing
/// throttle. Owning this value in code is what stops that regression returning.
const LIBERATE_INTERVAL_SECS: &str = "21600";

/// Image the workload runs. Pinning a tag is the follow-up; `latest` is what the
/// live deployment tracks today.
const IMAGE: &str = "rmcrackan/libation:latest";

/// libation backend. Holds only the provider name; per-instance address comes
/// from the `Routes` the generic `service.*` tools hand each op.
#[derive(Debug, Clone)]
pub struct LibationBackend {
    provider: &'static str,
}

impl LibationBackend {
    pub fn new(provider: &'static str) -> Self {
        Self { provider }
    }
}

fn env(key: &str, value: &str) -> EnvVar {
    EnvVar {
        key: key.to_string(),
        value: value.to_string(),
    }
}

impl ServiceBackend for LibationBackend {
    fn provider(&self) -> &str {
        self.provider
    }

    /// Runtimes libation can be placed on. `service.deploy` hands the
    /// `workload_spec` below to a matching deploy target — this backend never
    /// drives pct/docker itself (that mechanic lives in the deploy-target domain).
    fn runtimes(&self) -> Vec<Runtime> {
        vec![Runtime::Docker, Runtime::Podman, Runtime::Lxc]
    }

    fn capabilities(&self) -> Vec<ServiceCapability> {
        vec![
            ServiceCapability::Deploy,
            ServiceCapability::Backup,
            ServiceCapability::Restore,
            ServiceCapability::Configure,
            ServiceCapability::Status,
        ]
    }

    fn default_port(&self) -> u16 {
        9494
    }

    /// In-workload paths holding config/data. This is ALL libation declares for
    /// backup — the generic pluggable backup (tar for containers/LXC, PBS for
    /// Proxmox guests when available) snapshots these. No backup/restore code
    /// here; those are inherited from ServiceBackend's defaults.
    fn data_paths(&self) -> Vec<String> {
        vec!["/config".to_string()]
    }

    /// Container workload for libation. Host paths follow the appdata
    /// convention keyed on `instance`; making them config-driven is the
    /// follow-up (the signature carries no config access today).
    ///
    /// Note the spec cannot yet express the other half of the restart-loop fix —
    /// `WorkloadSpec` has no restart-policy field, and `Mount` has no bind
    /// propagation. Both need orca-side additions before a deploy from this spec
    /// fully matches the corrected compose.
    fn workload_spec<'a>(
        &'a self,
        runtime: Runtime,
        instance: &'a str,
        _routes: &'a Routes,
    ) -> BoxFuture<'a, Result<WorkloadSpec, ServiceError>> {
        Box::pin(async move {
            if !matches!(runtime, Runtime::Docker | Runtime::Podman) {
                return Err(ServiceError::unimplemented(
                    "libation.workload_spec: only Docker/Podman are described; \
                     an LXC template spec is not defined yet",
                ));
            }
            Ok(WorkloadSpec {
                name: instance.to_string(),
                image: Some(IMAGE.to_string()),
                env: vec![
                    env("SLEEP_TIME", LIBERATE_INTERVAL_SECS),
                    env("TZ", "America/Denver"),
                ],
                mounts: vec![
                    Mount::bind(format!("/opt/appdata/{instance}"), "/config"),
                    Mount::bind("/mnt/data/media/audiobooks-audible", "/data"),
                    Mount::bind(format!("/opt/appdata/{instance}-tmp"), "/tmp"),
                ],
                // Batch CLI workload — nothing to publish.
                ports: vec![],
            })
        })
    }

    fn configure<'a>(
        &'a self,
        _instance: &'a str,
        _routes: &'a Routes,
        _config: &'a str,
    ) -> BoxFuture<'a, Result<(), ServiceError>> {
        // TODO: apply libation-specific config idempotently.
        Box::pin(async move { Err(ServiceError::unimplemented("libation.configure")) })
    }

    fn status<'a>(
        &'a self,
        _instance: &'a str,
        _routes: &'a Routes,
    ) -> BoxFuture<'a, Result<ServiceStatus, ServiceError>> {
        // libation is a batch CLI with no health endpoint, so a real status has
        // to read container facts (restart count, last exit, run cadence) that
        // only the runtime adapter holds. Needs that seam before it is honest.
        Box::pin(async move { Err(ServiceError::unimplemented("libation.status")) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_provider() {
        let b = LibationBackend::new("libation");
        assert_eq!(b.provider(), "libation");
    }

    /// The regression guard: the interval must never be the run-once sentinel.
    #[test]
    fn liberate_interval_is_not_run_once() {
        assert_ne!(LIBERATE_INTERVAL_SECS, "-1");
        assert!(
            LIBERATE_INTERVAL_SECS
                .parse::<u32>()
                .is_ok_and(|s| s >= 3600),
            "interval must be numeric seconds and no hotter than hourly"
        );
    }

    #[tokio::test]
    async fn docker_spec_sets_a_sleeping_interval_and_all_three_mounts() {
        let b = LibationBackend::new("libation");
        let routes = Routes::default();
        let spec = b
            .workload_spec(Runtime::Docker, "libation", &routes)
            .await
            .expect("docker spec");

        assert_eq!(spec.name, "libation");
        assert_eq!(spec.image.as_deref(), Some(IMAGE));

        let sleep = spec
            .env
            .iter()
            .find(|e| e.key == "SLEEP_TIME")
            .expect("SLEEP_TIME present");
        assert_eq!(sleep.value, LIBERATE_INTERVAL_SECS);

        let targets: Vec<&str> = spec.mounts.iter().map(|m| m.target.as_str()).collect();
        assert_eq!(targets, vec!["/config", "/data", "/tmp"]);
        assert!(spec.ports.is_empty());
    }

    #[tokio::test]
    async fn lxc_spec_is_refused_rather_than_wrong() {
        let b = LibationBackend::new("libation");
        let routes = Routes::default();
        let err = b
            .workload_spec(Runtime::Lxc, "libation", &routes)
            .await
            .expect_err("lxc must not silently yield a container spec");
        assert!(format!("{err:?}").contains("workload_spec"));
    }
}
