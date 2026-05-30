use super::*;
use assert_matches::assert_matches;
use pretty_assertions::assert_eq;

fn profile_v2_for_args(args: &[&str]) -> anyhow::Result<Option<String>> {
    let cli = MultitoolCli::try_parse_from(args).expect("parse");
    let Some(subcommand) = cli.subcommand.as_ref() else {
        return Ok(cli
            .interactive
            .config_profile_v2
            .as_ref()
            .map(std::string::ToString::to_string));
    };
    Ok(profile_v2_for_subcommand(&cli.interactive, subcommand)?.map(ToString::to_string))
}

#[test]
fn profile_v2_is_rejected_for_config_management_subcommands() {
    assert!(profile_v2_for_args(&["codex", "--profile", "work", "features", "list"]).is_err());
}

#[test]
fn profile_v2_is_allowed_for_runtime_subcommands() {
    assert_eq!(
        profile_v2_for_args(&["codex", "--profile", "work", "resume"])
            .expect("resume supports profile-v2")
            .as_deref(),
        Some("work")
    );
    assert_eq!(
        profile_v2_for_args(&["codex", "--profile", "work", "debug", "prompt-input"])
            .expect("debug prompt-input supports profile-v2")
            .as_deref(),
        Some("work")
    );
    assert_eq!(
        profile_v2_for_args(&["codex", "--profile", "work", "mcp", "list"])
            .expect("mcp supports profile-v2")
            .as_deref(),
        Some("work")
    );
    assert_eq!(
        profile_v2_for_args(&["codex", "--profile", "work", "sandbox"])
            .expect("sandbox supports config profile")
            .as_deref(),
        Some("work")
    );
}

#[test]
fn profile_v2_rejects_non_plain_names_at_parse_time() {
    assert!(
        MultitoolCli::try_parse_from(["codex", "--profile", "nested/work", "resume"]).is_err()
    );
}

#[test]
fn exec_resume_last_accepts_prompt_positional() {
    let cli =
        MultitoolCli::try_parse_from(["codex", "exec", "--json", "resume", "--last", "2+2"])
            .expect("parse should succeed");

    let Some(Subcommand::Exec(exec)) = cli.subcommand else {
        panic!("expected exec subcommand");
    };
    let Some(codex_exec::Command::Resume(args)) = exec.command else {
        panic!("expected exec resume");
    };

    assert!(args.last);
    assert_eq!(args.session_id, None);
    assert_eq!(args.prompt.as_deref(), Some("2+2"));
}

#[test]
fn exec_resume_accepts_output_last_message_flag_after_subcommand() {
    let cli = MultitoolCli::try_parse_from([
        "codex",
        "exec",
        "resume",
        "session-123",
        "-o",
        "/tmp/resume-output.md",
        "re-review",
    ])
    .expect("parse should succeed");

    let Some(Subcommand::Exec(exec)) = cli.subcommand else {
        panic!("expected exec subcommand");
    };
    let Some(codex_exec::Command::Resume(args)) = exec.command else {
        panic!("expected exec resume");
    };

    assert_eq!(
        exec.last_message_file,
        Some(std::path::PathBuf::from("/tmp/resume-output.md"))
    );
    assert_eq!(args.session_id.as_deref(), Some("session-123"));
    assert_eq!(args.prompt.as_deref(), Some("re-review"));
}

#[test]
fn dangerous_bypass_conflicts_with_approval_policy() {
    let err = MultitoolCli::try_parse_from([
        "codex",
        "--dangerously-bypass-approvals-and-sandbox",
        "--ask-for-approval",
        "on-request",
    ])
    .expect_err("conflicting permission flags should be rejected");

    assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
}

fn app_server_from_args(args: &[&str]) -> AppServerCommand {
    let cli = MultitoolCli::try_parse_from(args).expect("parse");
    let Subcommand::AppServer(app_server) = cli.subcommand.expect("app-server present") else {
        unreachable!()
    };
    app_server
}

fn default_app_server_socket_path() -> AbsolutePathBuf {
    let codex_home = find_codex_home().expect("codex home");
    codex_app_server::app_server_control_socket_path(&codex_home)
        .expect("default app-server socket path")
}

#[test]
fn debug_prompt_input_parses_prompt_and_images() {
    let cli = MultitoolCli::try_parse_from([
        "codex",
        "debug",
        "prompt-input",
        "hello",
        "--image",
        "/tmp/a.png,/tmp/b.png",
    ])
    .expect("parse");

    let Some(Subcommand::Debug(DebugCommand {
        subcommand: DebugSubcommand::PromptInput(cmd),
    })) = cli.subcommand
    else {
        panic!("expected debug prompt-input subcommand");
    };

    assert_eq!(cmd.prompt.as_deref(), Some("hello"));
    assert_eq!(
        cmd.images,
        vec![PathBuf::from("/tmp/a.png"), PathBuf::from("/tmp/b.png")]
    );
}

#[test]
fn debug_models_parses_bundled_flag() {
    let cli =
        MultitoolCli::try_parse_from(["codex", "debug", "models", "--bundled"]).expect("parse");

    let Some(Subcommand::Debug(DebugCommand {
        subcommand: DebugSubcommand::Models(cmd),
    })) = cli.subcommand
    else {
        panic!("expected debug models subcommand");
    };

    assert!(cmd.bundled);
}

#[test]
fn debug_app_server_subcommand_is_not_registered() {
    let command = MultitoolCli::command();
    let debug_command = command
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "debug")
        .expect("debug subcommand should be registered");
    assert!(
        debug_command
            .get_subcommands()
            .all(|subcommand| subcommand.get_name() != "app-server")
    );
}

#[test]
fn responses_subcommand_is_not_registered() {
    let command = MultitoolCli::command();
    assert!(
        command
            .get_subcommands()
            .all(|subcommand| subcommand.get_name() != "responses")
    );
}

fn help_from_args(args: &[&str]) -> String {
    let err = MultitoolCli::try_parse_from(args).expect_err("help should short-circuit");
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    err.to_string()
}

#[test]
fn plugin_marketplace_help_uses_plugin_namespace() {
    let help = help_from_args(&["codex", "plugin", "marketplace", "--help"]);
    assert!(
        help.contains("Usage: codex plugin marketplace [OPTIONS] <COMMAND>"),
        "{help}"
    );

    for (subcommand, usage) in [
        ("add", "Usage: codex plugin marketplace add"),
        ("upgrade", "Usage: codex plugin marketplace upgrade"),
        ("remove", "Usage: codex plugin marketplace remove"),
    ] {
        let help = help_from_args(&["codex", "plugin", "marketplace", subcommand, "--help"]);
        assert!(help.contains(usage), "{help}");
    }
}

#[test]
fn plugin_marketplace_add_parses_under_plugin() {
    let cli =
        MultitoolCli::try_parse_from(["codex", "plugin", "marketplace", "add", "owner/repo"])
            .expect("parse");

    assert!(matches!(cli.subcommand, Some(Subcommand::Plugin(_))));
}

#[test]
fn plugin_marketplace_upgrade_parses_under_plugin() {
    let cli =
        MultitoolCli::try_parse_from(["codex", "plugin", "marketplace", "upgrade", "debug"])
            .expect("parse");

    assert!(matches!(cli.subcommand, Some(Subcommand::Plugin(_))));
}

#[test]
fn update_parses_as_update_subcommand() {
    let cli = MultitoolCli::try_parse_from(["codex", "update"]).expect("parse");
    assert!(matches!(cli.subcommand, Some(Subcommand::Update)));
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn sandbox_parses_permissions_profile() {
    let cli = MultitoolCli::try_parse_from([
        "codex",
        "sandbox",
        "--permissions-profile",
        ":workspace",
        "--",
        "echo",
    ])
    .expect("parse");

    let Some(Subcommand::Sandbox(command)) = cli.subcommand else {
        panic!("expected sandbox command");
    };

    assert_eq!(command.permissions_profile.as_deref(), Some(":workspace"));
    assert_eq!(command.command, vec!["echo"]);
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn sandbox_parses_config_profile() {
    let cli =
        MultitoolCli::try_parse_from(["codex", "sandbox", "--profile", "work", "--", "echo"])
            .expect("parse");

    let Some(Subcommand::Sandbox(command)) = cli.subcommand else {
        panic!("expected sandbox command");
    };

    assert_eq!(command.config_profile.as_deref(), Some("work"));
    assert_eq!(command.command, vec!["echo"]);
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
#[test]
fn sandbox_rejects_explicit_profile_controls_without_profile() {
    let err = MultitoolCli::try_parse_from(["codex", "sandbox", "-C", "/tmp"])
        .expect_err("parse should fail");

    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
}

#[test]
fn plugin_marketplace_remove_parses_under_plugin() {
    let cli =
        MultitoolCli::try_parse_from(["codex", "plugin", "marketplace", "remove", "debug"])
            .expect("parse");

    assert!(matches!(cli.subcommand, Some(Subcommand::Plugin(_))));
}

#[test]
fn marketplace_no_longer_parses_at_top_level() {
    let add_result =
        MultitoolCli::try_parse_from(["codex", "marketplace", "add", "owner/repo"]);
    assert!(add_result.is_err());

    let upgrade_result =
        MultitoolCli::try_parse_from(["codex", "marketplace", "upgrade", "debug"]);
    assert!(upgrade_result.is_err());

    let remove_result =
        MultitoolCli::try_parse_from(["codex", "marketplace", "remove", "debug"]);
    assert!(remove_result.is_err());
}

#[test]
fn full_auto_no_longer_parses_at_top_level() {
    let result = MultitoolCli::try_parse_from(["codex", "--full-auto"]);

    assert!(result.is_err());
}

#[test]
fn exec_full_auto_reports_migration_path() {
    let cli = MultitoolCli::try_parse_from(["codex", "exec", "--full-auto", "summarize"])
        .expect("exec should accept removed flag long enough to report a migration path");
    let Some(Subcommand::Exec(exec)) = cli.subcommand else {
        panic!("expected exec subcommand");
    };

    assert_eq!(
        exec.removed_full_auto_warning(),
        Some("warning: `--full-auto` is deprecated; use `--sandbox workspace-write` instead.")
    );
}

#[test]
fn sandbox_full_auto_no_longer_parses() {
    let result = MultitoolCli::try_parse_from(["codex", "sandbox", "--full-auto", "--"]);

    assert!(result.is_err());
}

#[test]
fn root_resume_flag_opens_picker_and_preserves_loop_settings() {
    let cli = MultitoolCli::try_parse_from([
        "codex",
        "--resume",
        "--loop",
        "--loop-period",
        "90s",
        "--loop-message",
        "continue",
    ])
    .expect("parse");

    assert!(cli.subcommand.is_none());
    assert!(cli.interactive.resume);
    assert!(cli.interactive.auto_loop);
    assert_eq!(
        cli.interactive.auto_loop_period,
        std::time::Duration::from_secs(90)
    );
    assert_eq!(cli.interactive.auto_loop_message, "continue");
}

#[test]
fn app_server_analytics_default_disabled_without_flag() {
    let app_server = app_server_from_args(["codex", "app-server"].as_ref());
    assert!(!app_server.analytics_default_enabled);
    assert!(!app_server.remote_control);
    assert_eq!(
        app_server.listen,
        codex_app_server::AppServerTransport::Stdio
    );
}

#[test]
fn app_server_analytics_default_enabled_with_flag() {
    let app_server =
        app_server_from_args(["codex", "app-server", "--analytics-default-enabled"].as_ref());
    assert!(app_server.analytics_default_enabled);
}

#[test]
fn strict_config_parses_for_supported_commands() {
    let cli = MultitoolCli::try_parse_from(["codex", "--strict-config"]).expect("parse");
    assert!(cli.interactive.strict_config);

    let cli = MultitoolCli::try_parse_from(["codex", "mcp-server", "--strict-config"])
        .expect("parse");
    assert_matches!(
        cli.subcommand,
        Some(Subcommand::McpServer(McpServerCommand {
            strict_config: true,
        }))
    );

    let cli =
        MultitoolCli::try_parse_from(["codex", "review", "--strict-config", "--uncommitted"])
            .expect("parse");
    assert_matches!(
        cli.subcommand,
        Some(Subcommand::Review(ReviewCommand {
            strict_config: true,
            ..
        }))
    );

    let cli = MultitoolCli::try_parse_from(["codex", "exec-server", "--strict-config"])
        .expect("parse");
    assert_matches!(
        cli.subcommand,
        Some(Subcommand::ExecServer(ExecServerCommand {
            strict_config: true,
            ..
        }))
    );
}

#[test]
fn root_strict_config_is_supported_for_exec_server() {
    let cli = MultitoolCli::try_parse_from(["codex", "--strict-config", "exec-server"])
        .expect("parse");

    reject_root_strict_config_for_subcommand(cli.interactive.strict_config, &cli.subcommand)
        .expect("exec-server should support root --strict-config");
}

#[test]
fn root_strict_config_is_rejected_for_unsupported_subcommands() {
    let cli = MultitoolCli::try_parse_from(["codex", "--strict-config", "mcp", "list"])
        .expect("parse");
    let err = reject_root_strict_config_for_subcommand(
        cli.interactive.strict_config,
        &cli.subcommand,
    )
    .expect_err("mcp should not support root --strict-config");

    assert_eq!(
        err.to_string(),
        "`--strict-config` is not supported for `codex mcp`"
    );

    let cli = MultitoolCli::try_parse_from(["codex", "--strict-config", "remote-control"])
        .expect("parse");
    let err = reject_root_strict_config_for_subcommand(
        cli.interactive.strict_config,
        &cli.subcommand,
    )
    .expect_err("remote-control should not support root --strict-config");

    assert_eq!(
        err.to_string(),
        "`--strict-config` is not supported for `codex remote-control`"
    );
}

#[test]
fn app_server_subcommands_reject_strict_config() {
    let app_server =
        app_server_from_args(["codex", "app-server", "--strict-config", "proxy"].as_ref());
    let err = reject_strict_config_for_app_server_subcommand(
        app_server.strict_config,
        app_server.subcommand.as_ref(),
    )
    .expect_err("app-server proxy should not support --strict-config");

    assert_eq!(
        err.to_string(),
        "`--strict-config` is not supported for `codex app-server proxy`"
    );
}

#[test]
fn reject_remote_flag_for_remote_control() {
    let cli = MultitoolCli::try_parse_from(["codex", "--remote", "unix://", "remote-control"])
        .expect("parse");
    assert_matches!(
        cli.subcommand,
        Some(Subcommand::RemoteControl(RemoteControlCommand {
            subcommand: None
        }))
    );

    let err = reject_remote_mode_for_subcommand(
        cli.remote.remote.as_deref(),
        cli.remote.remote_auth_token_env.as_deref(),
        "remote-control",
    )
    .expect_err("remote-control should reject root --remote");

    assert!(err.to_string().contains("remote-control"));
}

#[test]
fn remote_flag_parses_for_interactive_root() {
    let cli = MultitoolCli::try_parse_from(["codex", "--remote", "unix://codex.sock"])
        .expect("parse");
    assert_eq!(cli.remote.remote.as_deref(), Some("unix://codex.sock"));
}

#[test]
fn remote_auth_token_env_flag_parses_for_interactive_root() {
    let cli = MultitoolCli::try_parse_from([
        "codex",
        "--remote-auth-token-env",
        "CODEX_REMOTE_AUTH_TOKEN",
        "--remote",
        "ws://127.0.0.1:4500",
    ])
    .expect("parse");
    assert_eq!(
        cli.remote.remote_auth_token_env.as_deref(),
        Some("CODEX_REMOTE_AUTH_TOKEN")
    );
}

#[test]
fn remote_flag_parses_for_resume_subcommand() {
    let cli =
        MultitoolCli::try_parse_from(["codex", "resume", "--remote", "unix://codex.sock"])
            .expect("parse");
    let Subcommand::Resume(ResumeCommand { remote, .. }) =
        cli.subcommand.expect("resume present")
    else {
        panic!("expected resume subcommand");
    };
    assert_eq!(remote.remote.as_deref(), Some("unix://codex.sock"));
}

#[test]
fn reject_remote_mode_for_non_interactive_subcommands() {
    let err = reject_remote_mode_for_subcommand(
        Some("127.0.0.1:4500"),
        /*remote_auth_token_env*/ None,
        "exec",
    )
    .expect_err("non-interactive subcommands should reject --remote");
    assert!(
        err.to_string()
            .contains("only supported for interactive TUI commands")
    );
}

#[test]
fn reject_remote_auth_token_env_for_non_interactive_subcommands() {
    let err = reject_remote_mode_for_subcommand(
        /*remote*/ None,
        Some("CODEX_REMOTE_AUTH_TOKEN"),
        "exec",
    )
    .expect_err("non-interactive subcommands should reject --remote-auth-token-env");
    assert!(
        err.to_string()
            .contains("only supported for interactive TUI commands")
    );
}

#[test]
fn reject_remote_auth_token_env_for_app_server_generate_internal_json_schema() {
    let subcommand =
        AppServerSubcommand::GenerateInternalJsonSchema(GenerateInternalJsonSchemaCommand {
            out_dir: PathBuf::from("/tmp/out"),
        });
    let err = reject_remote_mode_for_app_server_subcommand(
        /*remote*/ None,
        Some("CODEX_REMOTE_AUTH_TOKEN"),
        Some(&subcommand),
    )
    .expect_err("non-interactive app-server subcommands should reject --remote-auth-token-env");
    assert!(err.to_string().contains("generate-internal-json-schema"));
}

#[test]
fn app_server_listen_websocket_url_parses() {
    let app_server = app_server_from_args(
        ["codex", "app-server", "--listen", "ws://127.0.0.1:4500"].as_ref(),
    );
    assert_eq!(
        app_server.listen,
        codex_app_server::AppServerTransport::WebSocket {
            bind_address: "127.0.0.1:4500".parse().expect("valid socket address"),
        }
    );
}

#[test]
fn app_server_listen_stdio_url_parses() {
    let app_server =
        app_server_from_args(["codex", "app-server", "--listen", "stdio://"].as_ref());
    assert_eq!(
        app_server.listen,
        codex_app_server::AppServerTransport::Stdio
    );
}

#[test]
fn app_server_listen_unix_socket_url_parses() {
    let app_server =
        app_server_from_args(["codex", "app-server", "--listen", "unix://"].as_ref());
    assert_eq!(
        app_server.listen,
        codex_app_server::AppServerTransport::UnixSocket {
            socket_path: default_app_server_socket_path()
        }
    );
}

#[test]
fn app_server_listen_unix_socket_path_parses() {
    let app_server = app_server_from_args(
        ["codex", "app-server", "--listen", "unix:///tmp/codex.sock"].as_ref(),
    );
    assert_eq!(
        app_server.listen,
        codex_app_server::AppServerTransport::UnixSocket {
            socket_path: AbsolutePathBuf::from_absolute_path("/tmp/codex.sock")
                .expect("absolute path should parse")
        }
    );
}

#[test]
fn app_server_listen_off_parses() {
    let app_server = app_server_from_args(["codex", "app-server", "--listen", "off"].as_ref());
    assert_eq!(app_server.listen, codex_app_server::AppServerTransport::Off);
}

#[test]
fn app_server_listen_invalid_url_fails_to_parse() {
    let parse_result =
        MultitoolCli::try_parse_from(["codex", "app-server", "--listen", "http://foo"]);
    assert!(parse_result.is_err());
}

#[test]
fn app_server_proxy_subcommand_parses() {
    let app_server = app_server_from_args(["codex", "app-server", "proxy"].as_ref());
    assert!(matches!(
        app_server.subcommand,
        Some(AppServerSubcommand::Proxy(AppServerProxyCommand {
            socket_path: None
        }))
    ));
}

#[test]
fn app_server_daemon_subcommands_parse() {
    assert!(matches!(
        app_server_from_args(
            [
                "codex",
                "app-server",
                "daemon",
                "bootstrap",
                "--remote-control"
            ]
            .as_ref()
        )
        .subcommand,
        Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
            subcommand: AppServerDaemonSubcommand::Bootstrap(AppServerBootstrapCommand {
                remote_control: true
            })
        }))
    ));
    assert!(matches!(
        app_server_from_args(["codex", "app-server", "daemon", "start"].as_ref()).subcommand,
        Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
            subcommand: AppServerDaemonSubcommand::Start
        }))
    ));
    assert!(matches!(
        app_server_from_args(["codex", "app-server", "daemon", "restart"].as_ref()).subcommand,
        Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
            subcommand: AppServerDaemonSubcommand::Restart
        }))
    ));
    assert!(matches!(
        app_server_from_args(
            ["codex", "app-server", "daemon", "enable-remote-control"].as_ref()
        )
        .subcommand,
        Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
            subcommand: AppServerDaemonSubcommand::EnableRemoteControl
        }))
    ));
    assert!(matches!(
        app_server_from_args(
            ["codex", "app-server", "daemon", "disable-remote-control"].as_ref()
        )
        .subcommand,
        Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
            subcommand: AppServerDaemonSubcommand::DisableRemoteControl
        }))
    ));
    assert!(matches!(
        app_server_from_args(["codex", "app-server", "daemon", "stop"].as_ref()).subcommand,
        Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
            subcommand: AppServerDaemonSubcommand::Stop
        }))
    ));
    assert!(matches!(
        app_server_from_args(["codex", "app-server", "daemon", "version"].as_ref()).subcommand,
        Some(AppServerSubcommand::Daemon(AppServerDaemonCommand {
            subcommand: AppServerDaemonSubcommand::Version
        }))
    ));
}

#[test]
fn app_server_proxy_sock_path_parses() {
    let app_server =
        app_server_from_args(["codex", "app-server", "proxy", "--sock", "codex.sock"].as_ref());
    let Some(AppServerSubcommand::Proxy(proxy)) = app_server.subcommand else {
        panic!("expected proxy subcommand");
    };
    assert_eq!(
        proxy.socket_path,
        Some(
            AbsolutePathBuf::relative_to_current_dir("codex.sock")
                .expect("relative path should resolve")
        )
    );
}

#[test]
fn reject_remote_auth_token_env_for_app_server_proxy() {
    let subcommand = AppServerSubcommand::Proxy(AppServerProxyCommand { socket_path: None });
    let err = reject_remote_mode_for_app_server_subcommand(
        /*remote*/ None,
        Some("CODEX_REMOTE_AUTH_TOKEN"),
        Some(&subcommand),
    )
    .expect_err("app-server proxy should reject --remote-auth-token-env");
    assert!(err.to_string().contains("app-server proxy"));
}

#[test]
fn reject_remote_auth_token_env_for_app_server_version() {
    let subcommand = AppServerSubcommand::Daemon(AppServerDaemonCommand {
        subcommand: AppServerDaemonSubcommand::Version,
    });
    let err = reject_remote_mode_for_app_server_subcommand(
        /*remote*/ None,
        Some("CODEX_REMOTE_AUTH_TOKEN"),
        Some(&subcommand),
    )
    .expect_err("app-server daemon version should reject --remote-auth-token-env");
    assert!(err.to_string().contains("app-server daemon version"));
}

#[test]
fn app_server_capability_token_flags_parse() {
    let app_server = app_server_from_args(
        [
            "codex",
            "app-server",
            "--ws-auth",
            "capability-token",
            "--ws-token-file",
            "/tmp/codex-token",
        ]
        .as_ref(),
    );
    assert_eq!(
        app_server.auth.ws_auth,
        Some(codex_app_server::WebsocketAuthCliMode::CapabilityToken)
    );
    assert_eq!(
        app_server.auth.ws_token_file,
        Some(PathBuf::from("/tmp/codex-token"))
    );
}

#[test]
fn app_server_signed_bearer_flags_parse() {
    let app_server = app_server_from_args(
        [
            "codex",
            "app-server",
            "--ws-auth",
            "signed-bearer-token",
            "--ws-shared-secret-file",
            "/tmp/codex-secret",
            "--ws-issuer",
            "issuer",
            "--ws-audience",
            "audience",
            "--ws-max-clock-skew-seconds",
            "9",
        ]
        .as_ref(),
    );
    assert_eq!(
        app_server.auth.ws_auth,
        Some(codex_app_server::WebsocketAuthCliMode::SignedBearerToken)
    );
    assert_eq!(
        app_server.auth.ws_shared_secret_file,
        Some(PathBuf::from("/tmp/codex-secret"))
    );
    assert_eq!(app_server.auth.ws_issuer.as_deref(), Some("issuer"));
    assert_eq!(app_server.auth.ws_audience.as_deref(), Some("audience"));
    assert_eq!(app_server.auth.ws_max_clock_skew_seconds, Some(9));
}

#[test]
fn app_server_rejects_removed_insecure_non_loopback_flag() {
    let parse_result = MultitoolCli::try_parse_from([
        "codex",
        "app-server",
        "--allow-unauthenticated-non-loopback-ws",
    ]);
    assert!(parse_result.is_err());
}

#[test]
fn features_enable_parses_feature_name() {
    let cli = MultitoolCli::try_parse_from(["codex", "features", "enable", "unified_exec"])
        .expect("parse should succeed");
    let Some(Subcommand::Features(FeaturesCli { sub })) = cli.subcommand else {
        panic!("expected features subcommand");
    };
    let FeaturesSubcommand::Enable(FeatureSetArgs { feature }) = sub else {
        panic!("expected features enable");
    };
    assert_eq!(feature, "unified_exec");
}

#[test]
fn features_disable_parses_feature_name() {
    let cli = MultitoolCli::try_parse_from(["codex", "features", "disable", "shell_tool"])
        .expect("parse should succeed");
    let Some(Subcommand::Features(FeaturesCli { sub })) = cli.subcommand else {
        panic!("expected features subcommand");
    };
    let FeaturesSubcommand::Disable(FeatureSetArgs { feature }) = sub else {
        panic!("expected features disable");
    };
    assert_eq!(feature, "shell_tool");
}

#[test]
fn feature_toggles_known_features_generate_overrides() {
    let toggles = FeatureToggles {
        enable: vec!["web_search_request".to_string()],
        disable: vec!["unified_exec".to_string()],
    };
    let overrides = toggles.to_overrides().expect("valid features");
    assert_eq!(
        overrides,
        vec![
            "features.web_search_request=true".to_string(),
            "features.unified_exec=false".to_string(),
        ]
    );
}

#[test]
fn feature_toggles_accept_legacy_linux_sandbox_flag() {
    let toggles = FeatureToggles {
        enable: vec!["use_linux_sandbox_bwrap".to_string()],
        disable: Vec::new(),
    };
    let overrides = toggles.to_overrides().expect("valid features");
    assert_eq!(
        overrides,
        vec!["features.use_linux_sandbox_bwrap=true".to_string(),]
    );
}

#[test]
fn feature_toggles_accept_removed_image_detail_original_flag() {
    let toggles = FeatureToggles {
        enable: vec!["image_detail_original".to_string()],
        disable: Vec::new(),
    };
    let overrides = toggles.to_overrides().expect("valid features");
    assert_eq!(
        overrides,
        vec!["features.image_detail_original=true".to_string(),]
    );
}

#[test]
fn feature_toggles_unknown_feature_errors() {
    let toggles = FeatureToggles {
        enable: vec!["does_not_exist".to_string()],
        disable: Vec::new(),
    };
    let err = toggles
        .to_overrides()
        .expect_err("feature should be rejected");
    assert_eq!(err.to_string(), "Unknown feature flag: does_not_exist");
}

#[test]
fn strict_config_with_unknown_enable_errors() {
    let err = strict_config_feature_toggle_error(["--enable", "does_not_exist"].as_ref());
    assert_eq!(err.to_string(), "Unknown feature flag: does_not_exist");
}

#[test]
fn strict_config_with_unknown_disable_errors() {
    let err = strict_config_feature_toggle_error(["--disable", "does_not_exist"].as_ref());
    assert_eq!(err.to_string(), "Unknown feature flag: does_not_exist");
}

#[test]
fn strict_config_with_compound_enable_errors() {
    let err = strict_config_feature_toggle_error(
        ["--enable", "multi_agent_v2.subagent_usage_hint_text"].as_ref(),
    );
    assert_eq!(
        err.to_string(),
        "Unknown feature flag: multi_agent_v2.subagent_usage_hint_text"
    );
}

fn strict_config_feature_toggle_error(args: &[&str]) -> anyhow::Error {
    let cli_args = std::iter::once("codex")
        .chain(std::iter::once("--strict-config"))
        .chain(args.iter().copied());
    let cli = MultitoolCli::try_parse_from(cli_args).expect("parse should succeed");
    assert!(cli.interactive.strict_config);
    cli.feature_toggles
        .to_overrides()
        .expect_err("feature should be rejected")
}
