#![allow(clippy::disallowed_types)]

pub mod tools;

use plugin_toolkit::clap;
use plugin_toolkit::service::{
    BoxFuture, Routes, Runtime, ServiceBackend, ServiceCapability, ServiceError, ServiceStatus,
    WorkloadSpec,
};

#[plugin_toolkit::endpoint_resource(plugin = "libation")]
pub struct LibationEndpoint {
    pub name: String,
    pub export_path: String,
}

#[derive(Debug, Clone)]
pub struct LibationBackend {
    provider: &'static str,
}

impl LibationBackend {
    pub fn new(provider: &'static str) -> Self {
        Self { provider }
    }
}

impl ServiceBackend for LibationBackend {
    fn provider(&self) -> &str {
        self.provider
    }
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
    fn data_paths(&self) -> Vec<String> {
        vec!["/config".to_string()]
    }

    fn workload_spec<'a>(
        &'a self,
        _runtime: Runtime,
        _instance: &'a str,
        _routes: &'a Routes,
    ) -> BoxFuture<'a, Result<WorkloadSpec, ServiceError>> {
        Box::pin(async move { Err(ServiceError::unimplemented("libation.workload_spec")) })
    }

    fn configure<'a>(
        &'a self,
        _instance: &'a str,
        _routes: &'a Routes,
        _config: &'a str,
    ) -> BoxFuture<'a, Result<(), ServiceError>> {
        Box::pin(async move { Err(ServiceError::unimplemented("libation.configure")) })
    }

    fn status<'a>(
        &'a self,
        _instance: &'a str,
        _routes: &'a Routes,
    ) -> BoxFuture<'a, Result<ServiceStatus, ServiceError>> {
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
}
