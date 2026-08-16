use crate::error::ClsResult;

/// Sandbox de ejecución: restringe acceso a recursos
pub struct Sandbox {
    pub allow_fs: bool,
    pub allow_net: bool,
    pub max_execution_time_ms: u64,
}

impl Sandbox {
    pub fn new() -> Self {
        Self {
            allow_fs: false,
            allow_net: false,
            max_execution_time_ms: 5000,
        }
    }

    pub fn from_config(config: &cls_core::config::SandboxConfig) -> Self {
        Self {
            allow_fs: config.allow_fs,
            allow_net: config.allow_net,
            max_execution_time_ms: config.max_execution_time,
        }
    }

    pub fn check_fs_access(&self) -> ClsResult<()> {
        if !self.allow_fs {
            return Err(crate::error::ClsError::RuntimeError(
                "Acceso a FS denegado por sandbox".to_string(),
            ));
        }
        Ok(())
    }

    pub fn check_net_access(&self) -> ClsResult<()> {
        if !self.allow_net {
            return Err(crate::error::ClsError::RuntimeError(
                "Acceso a red denegado por sandbox".to_string(),
            ));
        }
        Ok(())
    }
}
