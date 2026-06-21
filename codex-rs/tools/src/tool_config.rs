use codex_features::Feature;
use codex_features::Features;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::ModelInfo;

pub use codex_tool_execution_api::ShellCommandBackendConfig;
pub use codex_tool_execution_api::ToolEnvironmentMode;
pub use codex_tool_execution_api::ToolUserShellType;
pub use codex_tool_execution_api::ToolsConfig;
pub use codex_tool_execution_api::ToolsConfigParams;
pub use codex_tool_execution_api::UnifiedExecShellMode;
pub use codex_tool_execution_api::ZshForkConfig;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UnifiedExecFeatureMode {
    /// Unified exec should not be selected by this feature set.
    ///
    /// This includes standalone `shell_zsh_fork`: until
    /// `unified_exec_zsh_fork` is enabled too, `shell_zsh_fork` keeps using
    /// the shell command backend instead of silently opting unified exec into
    /// zsh-fork interception.
    Disabled,
    Direct,
    ZshFork,
}

pub fn shell_command_backend_for_features(features: &Features) -> ShellCommandBackendConfig {
    if features.enabled(Feature::ShellTool) && features.enabled(Feature::ShellZshFork) {
        ShellCommandBackendConfig::ZshFork
    } else {
        ShellCommandBackendConfig::Classic
    }
}

/// Returns the unified-exec mode requested by feature policy, before runtime
/// session inputs such as platform, user shell, and zsh-fork binary paths are
/// resolved.
///
/// `unified_exec_zsh_fork` is only a composition gate. It does not enable
/// either underlying shell mode on its own, so disabling `unified_exec` or
/// `shell_zsh_fork` keeps those features independently off. This lets
/// enterprise deployments opt into, or out of, unified exec and zsh-fork
/// behavior separately; otherwise enabling the composition flag would silently
/// activate a shell backend that the configured feature set left disabled.
pub fn unified_exec_feature_mode_for_features(features: &Features) -> UnifiedExecFeatureMode {
    if !features.enabled(Feature::ShellTool) || !features.enabled(Feature::UnifiedExec) {
        UnifiedExecFeatureMode::Disabled
    } else if features.enabled(Feature::ShellZshFork) {
        if features.enabled(Feature::UnifiedExecZshFork) {
            UnifiedExecFeatureMode::ZshFork
        } else {
            UnifiedExecFeatureMode::Disabled
        }
    } else {
        UnifiedExecFeatureMode::Direct
    }
}

pub fn shell_type_for_model_and_features(
    model_info: &ModelInfo,
    features: &Features,
) -> ConfigShellToolType {
    let unified_exec_feature_mode = unified_exec_feature_mode_for_features(features);
    let unified_exec_disabled =
        matches!(unified_exec_feature_mode, UnifiedExecFeatureMode::Disabled);
    let model_shell_type = match model_info.shell_type {
        ConfigShellToolType::UnifiedExec if unified_exec_disabled => {
            ConfigShellToolType::ShellCommand
        }
        ConfigShellToolType::Default | ConfigShellToolType::Local => {
            ConfigShellToolType::ShellCommand
        }
        other => other,
    };
    let shell_command_type = match shell_command_backend_for_features(features) {
        ShellCommandBackendConfig::Classic => model_shell_type,
        ShellCommandBackendConfig::ZshFork => ConfigShellToolType::ShellCommand,
    };

    if !features.enabled(Feature::ShellTool) {
        ConfigShellToolType::Disabled
    } else {
        match unified_exec_feature_mode {
            UnifiedExecFeatureMode::Disabled => shell_command_type,
            UnifiedExecFeatureMode::Direct | UnifiedExecFeatureMode::ZshFork => {
                if codex_utils_pty::conpty_supported() {
                    ConfigShellToolType::UnifiedExec
                } else {
                    ConfigShellToolType::ShellCommand
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tool_config_tests.rs"]
mod tests;
