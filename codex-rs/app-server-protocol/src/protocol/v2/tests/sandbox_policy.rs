use super::*;
use pretty_assertions::assert_eq;

#[test]
fn sandbox_policy_round_trips_external_sandbox_network_access() {
    let v2_policy = SandboxPolicy::ExternalSandbox {
        network_access: NetworkAccess::Enabled,
    };

    let core_policy = v2_policy.to_core();
    assert_eq!(
        core_policy,
        codex_protocol::protocol::SandboxPolicy::ExternalSandbox {
            network_access: CoreNetworkAccess::Enabled,
        }
    );

    let back_to_v2 = SandboxPolicy::from(core_policy);
    assert_eq!(back_to_v2, v2_policy);
}

#[test]
fn sandbox_policy_round_trips_read_only_network_access() {
    let v2_policy = SandboxPolicy::ReadOnly {
        network_access: true,
    };

    let core_policy = v2_policy.to_core();
    assert_eq!(
        core_policy,
        codex_protocol::protocol::SandboxPolicy::ReadOnly {
            network_access: true,
        }
    );

    let back_to_v2 = SandboxPolicy::from(core_policy);
    assert_eq!(back_to_v2, v2_policy);
}

#[test]
fn sandbox_policy_round_trips_workspace_write_access() {
    let v2_policy = SandboxPolicy::WorkspaceWrite {
        writable_roots: vec![],
        network_access: true,
        exclude_tmpdir_env_var: false,
        exclude_slash_tmp: false,
    };

    let core_policy = v2_policy.to_core();
    assert_eq!(
        core_policy,
        codex_protocol::protocol::SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![],
            network_access: true,
            exclude_tmpdir_env_var: false,
            exclude_slash_tmp: false,
        }
    );

    let back_to_v2 = SandboxPolicy::from(core_policy);
    assert_eq!(back_to_v2, v2_policy);
}

#[test]
fn sandbox_policy_deserializes_legacy_read_only_full_access_field() {
    let policy = serde_json::from_value::<SandboxPolicy>(json!({
        "type": "readOnly",
        "access": {
            "type": "fullAccess"
        },
        "networkAccess": true
    }))
    .expect("read-only policy should ignore legacy fullAccess field");
    assert_eq!(
        policy,
        SandboxPolicy::ReadOnly {
            network_access: true
        }
    );
}

#[test]
fn sandbox_policy_deserializes_legacy_workspace_write_full_access_field() {
    let writable_root = absolute_path("/workspace");
    let policy = serde_json::from_value::<SandboxPolicy>(json!({
        "type": "workspaceWrite",
        "writableRoots": [writable_root],
        "readOnlyAccess": {
            "type": "fullAccess"
        },
        "networkAccess": true,
        "excludeTmpdirEnvVar": true,
        "excludeSlashTmp": true
    }))
    .expect("workspace-write policy should ignore legacy fullAccess field");
    assert_eq!(
        policy,
        SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![absolute_path("/workspace")],
            network_access: true,
            exclude_tmpdir_env_var: true,
            exclude_slash_tmp: true,
        }
    );
}

#[test]
fn sandbox_policy_rejects_legacy_read_only_restricted_access_field() {
    let err = serde_json::from_value::<SandboxPolicy>(json!({
        "type": "readOnly",
        "access": {
            "type": "restricted",
            "includePlatformDefaults": false,
            "readableRoots": []
        }
    }))
    .expect_err("read-only policy should reject removed restricted access field");
    assert!(err.to_string().contains("readOnly.access"));
}

#[test]
fn sandbox_policy_rejects_legacy_workspace_write_restricted_read_access_field() {
    let err = serde_json::from_value::<SandboxPolicy>(json!({
        "type": "workspaceWrite",
        "writableRoots": [],
        "readOnlyAccess": {
            "type": "restricted",
            "includePlatformDefaults": false,
            "readableRoots": []
        },
        "networkAccess": false,
        "excludeTmpdirEnvVar": false,
        "excludeSlashTmp": false
    }))
    .expect_err("workspace-write policy should reject removed restricted readOnlyAccess field");
    assert!(err.to_string().contains("workspaceWrite.readOnlyAccess"));
}
