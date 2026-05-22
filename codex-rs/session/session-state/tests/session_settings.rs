use std::collections::BTreeMap;
use std::path::PathBuf;

use codex_session_state::PreviousTurnSettings;
use codex_session_state::SessionEnvironmentState;
use codex_session_state::SessionEnvironmentUpdate;
use codex_session_state::SessionServiceTierUpdate;
use codex_session_state::SessionSettingsSnapshot;
use codex_session_state::SessionSettingsUpdate;
use codex_session_state::SessionWorkspaceRoots;
use codex_session_state::SessionWorkspaceRootsUpdate;

fn path(value: &str) -> PathBuf {
    PathBuf::from(value)
}

fn settings_snapshot() -> SessionSettingsSnapshot {
    let mut snapshot = SessionSettingsSnapshot::new(SessionWorkspaceRoots::new(
        path("C:/repo"),
        vec![path("C:/repo")],
        vec![path("C:/profile-root")],
    ));
    snapshot.service_tier = Some("flex".to_string());
    snapshot.environment = SessionEnvironmentState::new(vec!["web".to_string()]);
    snapshot.config_metadata = BTreeMap::from([("source".to_string(), "project".to_string())]);
    snapshot
}

#[test]
fn previous_turn_settings_carry_model_and_realtime_state() {
    let settings = PreviousTurnSettings::new("gpt-5.4".to_string(), Some(false));

    assert_eq!(
        settings,
        PreviousTurnSettings {
            model: "gpt-5.4".to_string(),
            realtime_active: Some(false),
        }
    );
}

#[test]
fn settings_update_preserves_workspace_roots_when_omitted() {
    let mut snapshot = settings_snapshot();

    snapshot.apply_update(SessionSettingsUpdate {
        service_tier: SessionServiceTierUpdate::Set("priority".to_string()),
        ..Default::default()
    });

    assert_eq!(snapshot.workspace_roots.cwd, path("C:/repo"));
    assert_eq!(
        snapshot.workspace_roots.runtime_workspace_roots,
        vec![path("C:/repo")]
    );
    assert_eq!(
        snapshot.workspace_roots.profile_workspace_roots,
        vec![path("C:/profile-root")]
    );
    assert_eq!(snapshot.service_tier, Some("priority".to_string()));
}

#[test]
fn workspace_root_update_replaces_runtime_roots_without_dropping_profile_roots() {
    let mut snapshot = settings_snapshot();

    snapshot.apply_update(SessionSettingsUpdate {
        workspace_roots: Some(SessionWorkspaceRootsUpdate {
            runtime_workspace_roots: Some(vec![path("D:/work"), path("E:/shared")]),
            ..Default::default()
        }),
        ..Default::default()
    });

    assert_eq!(
        snapshot.workspace_roots.runtime_workspace_roots,
        vec![path("D:/work"), path("E:/shared")]
    );
    assert_eq!(
        snapshot.workspace_roots.profile_workspace_roots,
        vec![path("C:/profile-root")]
    );
}

#[test]
fn service_tier_update_distinguishes_preserve_set_and_clear() {
    let mut snapshot = settings_snapshot();

    snapshot.apply_update(SessionSettingsUpdate::default());
    assert_eq!(snapshot.service_tier, Some("flex".to_string()));

    snapshot.apply_update(SessionSettingsUpdate {
        service_tier: SessionServiceTierUpdate::Set("priority".to_string()),
        ..Default::default()
    });
    assert_eq!(snapshot.service_tier, Some("priority".to_string()));

    snapshot.apply_update(SessionSettingsUpdate {
        service_tier: SessionServiceTierUpdate::Clear,
        ..Default::default()
    });
    assert_eq!(snapshot.service_tier, None);
}

#[test]
fn environment_update_distinguishes_preserve_disable_and_replace() {
    let mut snapshot = settings_snapshot();

    snapshot.apply_update(SessionSettingsUpdate::default());
    assert_eq!(
        snapshot.environment.selected_environments,
        vec!["web".to_string()]
    );

    snapshot.apply_update(SessionSettingsUpdate {
        environment: SessionEnvironmentUpdate::Disable,
        ..Default::default()
    });
    assert_eq!(
        snapshot.environment.selected_environments,
        Vec::<String>::new()
    );

    snapshot.apply_update(SessionSettingsUpdate {
        environment: SessionEnvironmentUpdate::Replace(vec![
            "web".to_string(),
            "browser".to_string(),
        ]),
        ..Default::default()
    });
    assert_eq!(
        snapshot.environment.selected_environments,
        vec!["web".to_string(), "browser".to_string()]
    );
}

#[test]
fn config_metadata_update_replaces_previous_config_state() {
    let mut snapshot = settings_snapshot();

    snapshot.apply_update(SessionSettingsUpdate {
        config_metadata: Some(BTreeMap::from([
            ("source".to_string(), "cloud".to_string()),
            ("requirements".to_string(), "network".to_string()),
        ])),
        ..Default::default()
    });

    assert_eq!(
        snapshot.config_metadata,
        BTreeMap::from([
            ("requirements".to_string(), "network".to_string()),
            ("source".to_string(), "cloud".to_string()),
        ])
    );
}
