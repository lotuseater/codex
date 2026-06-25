//! Implements the `codex exec-server` subcommand dispatch and its helpers.
//!
//! The handler body and its auth/config loaders are moved verbatim from
//! `main.rs`. Behavior, validation order, and output are unchanged.

use codex_arg0::Arg0DispatchPaths;
use codex_login::CodexAuth;
use codex_login::read_codex_access_token_from_env;
use codex_utils_cli::CliConfigOverrides;

use codex_core::config::ConfigBuilder;
use codex_login::AuthManager;

use crate::ExecServerCommand;

pub(crate) async fn run_exec_server_command(
    cmd: ExecServerCommand,
    arg0_paths: &Arg0DispatchPaths,
    root_config_overrides: &CliConfigOverrides,
    strict_config: bool,
) -> anyhow::Result<()> {
    let codex_self_exe = arg0_paths
        .codex_self_exe
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Codex executable path is not configured"))?;
    let runtime_paths = codex_exec_server::ExecServerRuntimePaths::new(
        codex_self_exe,
        arg0_paths.codex_linux_sandbox_exe.clone(),
    )?;
    if let Some(base_url) = cmd.remote {
        let environment_id = cmd
            .environment_id
            .ok_or_else(|| anyhow::anyhow!("--environment-id is required when --remote is set"))?;
        let config = load_exec_server_config(root_config_overrides, strict_config).await?;
        let auth_provider =
            load_exec_server_remote_auth_provider(&config, cmd.use_agent_identity_auth).await?;
        let mut remote_config = codex_exec_server::RemoteEnvironmentConfig::new(
            base_url,
            environment_id,
            auth_provider,
        )?;
        if let Some(name) = cmd.name {
            remote_config.name = name;
        }
        codex_exec_server::run_remote_environment(remote_config, runtime_paths).await?;
        Ok(())
    } else {
        if strict_config {
            // Local exec-server startup does not consume Config, but strict
            // mode should still reject unknown fields before opening a listener.
            let _validated_config =
                load_exec_server_config(root_config_overrides, strict_config).await?;
        }
        let listen_url = cmd
            .listen
            .as_deref()
            .unwrap_or(codex_exec_server::DEFAULT_LISTEN_URL);
        codex_exec_server::run_main(listen_url, runtime_paths)
            .await
            .map_err(anyhow::Error::from_boxed)
    }
}

async fn load_exec_server_remote_auth_provider(
    config: &codex_core::config::Config,
    use_agent_identity_auth: bool,
) -> anyhow::Result<codex_api::SharedAuthProvider> {
    if use_agent_identity_auth {
        let agent_identity_jwt = read_codex_access_token_from_env().ok_or_else(|| {
            anyhow::anyhow!("CODEX_ACCESS_TOKEN is required when --use-agent-identity-auth is set")
        })?;
        let auth_route_config = config.auth_route_config();
        let auth = CodexAuth::from_agent_identity_jwt(
            &agent_identity_jwt,
            Some(&config.chatgpt_base_url),
            auth_route_config.as_ref(),
        )
        .await?;
        return Ok(codex_model_provider::auth_provider_from_auth(&auth));
    }

    let auth = load_exec_server_remote_auth(
        config,
        "remote exec-server registration requires ChatGPT authentication; run `codex login` first",
    )
    .await?;

    if !auth.is_chatgpt_auth() {
        anyhow::bail!(
            "remote exec-server registration requires ChatGPT authentication; API key and Agent Identity auth are not supported"
        );
    }

    Ok(codex_model_provider::auth_provider_from_auth(&auth))
}

async fn load_exec_server_config(
    root_config_overrides: &CliConfigOverrides,
    strict_config: bool,
) -> anyhow::Result<codex_core::config::Config> {
    let cli_kv_overrides = root_config_overrides
        .parse_overrides()
        .map_err(anyhow::Error::msg)?;
    Ok(ConfigBuilder::default()
        .cli_overrides(cli_kv_overrides)
        .strict_config(strict_config)
        .build()
        .await?)
}

async fn load_exec_server_remote_auth(
    config: &codex_core::config::Config,
    missing_auth_error: &'static str,
) -> anyhow::Result<codex_login::CodexAuth> {
    let auth_manager =
        AuthManager::shared_from_config(config, /*enable_codex_api_key_env*/ true).await;

    let auth = match auth_manager.auth().await {
        Some(auth) => auth,
        None => {
            auth_manager.reload().await;
            auth_manager
                .auth()
                .await
                .ok_or_else(|| anyhow::anyhow!(missing_auth_error))?
        }
    };

    Ok(auth)
}
