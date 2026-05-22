#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::approvals_support::*;

pub(crate) fn scenarios() -> Vec<ScenarioSpec> {
    use AskForApproval::*;

    let workspace_write = |network_access| SandboxPolicy::WorkspaceWrite {
        writable_roots: vec![],
        network_access,
        exclude_tmpdir_env_var: false,
        exclude_slash_tmp: false,
    };

    vec![
        ScenarioSpec {
            name: "danger_full_access_on_request_allows_outside_write",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("dfa_on_request.txt"),
                content: "danger-on-request",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::FileCreated {
                target: TargetPath::OutsideWorkspace("dfa_on_request.txt"),
                content: "danger-on-request",
            },
        },
        ScenarioSpec {
            name: "danger_full_access_on_request_allows_outside_write_gpt_5_1_no_exit",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("dfa_on_request_5_1.txt"),
                content: "danger-on-request",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::Auto,
            expectation: Expectation::FileCreated {
                target: TargetPath::OutsideWorkspace("dfa_on_request_5_1.txt"),
                content: "danger-on-request",
            },
        },
        ScenarioSpec {
            name: "danger_full_access_on_request_allows_network",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::FetchUrlNoProxy {
                endpoint: "/dfa/network",
                response_body: "danger-network-ok",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::NetworkSuccess {
                body_contains: "danger-network-ok",
            },
        },
        ScenarioSpec {
            name: "danger_full_access_on_request_allows_network_gpt_5_1_no_exit",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::FetchUrlNoProxy {
                endpoint: "/dfa/network",
                response_body: "danger-network-ok",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::Auto,
            expectation: Expectation::NetworkSuccessNoExitCode {
                body_contains: "danger-network-ok",
            },
        },
        ScenarioSpec {
            name: "trusted_command_unless_trusted_runs_without_prompt",
            approval_policy: UnlessTrusted,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::RunCommand {
                command: "echo trusted-unless",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::CommandSuccess {
                stdout_contains: "trusted-unless",
            },
        },
        ScenarioSpec {
            name: "trusted_command_unless_trusted_runs_without_prompt_gpt_5_1_no_exit",
            approval_policy: UnlessTrusted,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::RunCommand {
                command: "echo trusted-unless",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::Auto,
            expectation: Expectation::CommandSuccessNoExitCode {
                stdout_contains: "trusted-unless",
            },
        },
        ScenarioSpec {
            name: "cat_redirect_unless_trusted_requires_approval",
            approval_policy: UnlessTrusted,
            sandbox_policy: workspace_write(false),
            action: ActionKind::RunCommand {
                command: r#"cat < "hello" > /var/test.txt"#,
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Denied,
                expected_reason: None,
            },
            expectation: Expectation::CommandFailure {
                output_contains: "rejected by user",
            },
        },
        ScenarioSpec {
            name: "cat_redirect_on_request_requires_approval",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::RunCommand {
                command: r#"cat < "hello" > /var/test.txt"#,
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Denied,
                expected_reason: None,
            },
            expectation: Expectation::CommandFailure {
                output_contains: "rejected by user",
            },
        },
        ScenarioSpec {
            name: "known_safe_escalation_on_request_requires_approval",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::RunCommand {
                command: "echo known-safe-escalation",
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApprovalWithAmendment {
                decision: ReviewDecision::Denied,
                expected_reason: None,
                expected_execpolicy_amendment: Some(&["echo", "known-safe-escalation"]),
            },
            expectation: Expectation::CommandFailure {
                output_contains: "rejected by user",
            },
        },
        ScenarioSpec {
            name: "known_safe_escalation_granular_sandbox_disabled_rejects",
            approval_policy: Granular(GranularApprovalConfig {
                sandbox_approval: false,
                rules: true,
                skill_approval: true,
                request_permissions: true,
                mcp_elicitations: true,
            }),
            sandbox_policy: workspace_write(false),
            action: ActionKind::RunCommand {
                command: "echo known-safe-escalation-granular-disabled",
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::CommandFailure {
                output_contains: "you should not ask for escalated permissions",
            },
        },
        ScenarioSpec {
            name: "cat_heredoc_file_redirect_prefix_rule_requires_escalation_approval",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::RunCommandWithPrefixRule {
                command: r#"cat <<'EOF' > /tmp/out.txt
                hello
                EOF"#,
                prefix_rule: &["cat"],
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Denied,
                expected_reason: None,
            },
            expectation: Expectation::CommandFailure {
                output_contains: "rejected by user",
            },
        },
        ScenarioSpec {
            name: "cat_heredoc_variable_assignment_policy_requires_escalation_approval",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::RunCommandWithPolicy {
                command: r#"PATH=/tmp/evil:$PATH cat <<'EOF'
                hello
                EOF"#,
                policy_src: r#"prefix_rule(pattern=["cat"], decision="allow")"#,
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Denied,
                expected_reason: None,
            },
            expectation: Expectation::CommandFailure {
                output_contains: "rejected by user",
            },
        },
        ScenarioSpec {
            name: "python_heredoc_requested_prefix_rule_omits_amendment",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::RunCommandWithPrefixRule {
                command: r#"python3 <<'PY'
                print('hello')
                PY"#,
                prefix_rule: &["python3"],
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApprovalWithAmendment {
                decision: ReviewDecision::Denied,
                expected_reason: None,
                expected_execpolicy_amendment: None,
            },
            expectation: Expectation::CommandFailure {
                output_contains: "rejected by user",
            },
        },
        ScenarioSpec {
            name: "danger_full_access_on_failure_allows_outside_write",
            approval_policy: OnFailure,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("dfa_on_failure.txt"),
                content: "danger-on-failure",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::FileCreated {
                target: TargetPath::OutsideWorkspace("dfa_on_failure.txt"),
                content: "danger-on-failure",
            },
        },
        ScenarioSpec {
            name: "danger_full_access_on_failure_allows_outside_write_gpt_5_1_no_exit",
            approval_policy: OnFailure,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("dfa_on_failure_5_1.txt"),
                content: "danger-on-failure",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::Auto,
            expectation: Expectation::FileCreatedNoExitCode {
                target: TargetPath::OutsideWorkspace("dfa_on_failure_5_1.txt"),
                content: "danger-on-failure",
            },
        },
        ScenarioSpec {
            name: "danger_full_access_unless_trusted_requests_approval",
            approval_policy: UnlessTrusted,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("dfa_unless_trusted.txt"),
                content: "danger-unless-trusted",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::FileCreated {
                target: TargetPath::OutsideWorkspace("dfa_unless_trusted.txt"),
                content: "danger-unless-trusted",
            },
        },
        ScenarioSpec {
            name: "danger_full_access_unless_trusted_requests_approval_gpt_5_1_no_exit",
            approval_policy: UnlessTrusted,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("dfa_unless_trusted_5_1.txt"),
                content: "danger-unless-trusted",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::FileCreatedNoExitCode {
                target: TargetPath::OutsideWorkspace("dfa_unless_trusted_5_1.txt"),
                content: "danger-unless-trusted",
            },
        },
        ScenarioSpec {
            name: "danger_full_access_never_allows_outside_write",
            approval_policy: Never,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("dfa_never.txt"),
                content: "danger-never",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::FileCreated {
                target: TargetPath::OutsideWorkspace("dfa_never.txt"),
                content: "danger-never",
            },
        },
        ScenarioSpec {
            name: "danger_full_access_never_allows_outside_write_gpt_5_1_no_exit",
            approval_policy: Never,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("dfa_never_5_1.txt"),
                content: "danger-never",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::Auto,
            expectation: Expectation::FileCreatedNoExitCode {
                target: TargetPath::OutsideWorkspace("dfa_never_5_1.txt"),
                content: "danger-never",
            },
        },
        ScenarioSpec {
            name: "read_only_on_request_requires_approval",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::WriteFile {
                target: TargetPath::Workspace("ro_on_request.txt"),
                content: "read-only-approval",
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::FileCreated {
                target: TargetPath::Workspace("ro_on_request.txt"),
                content: "read-only-approval",
            },
        },
        ScenarioSpec {
            name: "read_only_on_request_requires_approval_gpt_5_1_no_exit",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::WriteFile {
                target: TargetPath::Workspace("ro_on_request_5_1.txt"),
                content: "read-only-approval",
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::FileCreatedNoExitCode {
                target: TargetPath::Workspace("ro_on_request_5_1.txt"),
                content: "read-only-approval",
            },
        },
        ScenarioSpec {
            name: "trusted_command_on_request_read_only_runs_without_prompt",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::RunCommand {
                command: "echo trusted-read-only",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::CommandSuccess {
                stdout_contains: "trusted-read-only",
            },
        },
        ScenarioSpec {
            name: "trusted_command_on_request_read_only_runs_without_prompt_gpt_5_1_no_exit",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::RunCommand {
                command: "echo trusted-read-only",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::Auto,
            expectation: Expectation::CommandSuccessNoExitCode {
                stdout_contains: "trusted-read-only",
            },
        },
        ScenarioSpec {
            name: "read_only_on_request_blocks_network",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::FetchUrl {
                endpoint: "/ro/network-blocked",
                response_body: "should-not-see",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: None,
            outcome: Outcome::Auto,
            expectation: Expectation::NetworkFailure { expect_tag: "ERR:" },
        },
        ScenarioSpec {
            name: "read_only_on_request_denied_blocks_execution",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::WriteFile {
                target: TargetPath::Workspace("ro_on_request_denied.txt"),
                content: "should-not-write",
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: None,
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Denied,
                expected_reason: None,
            },
            expectation: Expectation::FileNotCreated {
                target: TargetPath::Workspace("ro_on_request_denied.txt"),
                message_contains: &["exec command rejected by user"],
            },
        },
        #[cfg(not(target_os = "linux"))] // TODO (pakrym): figure out why linux behaves differently
        ScenarioSpec {
            name: "read_only_on_failure_escalates_after_sandbox_error",
            approval_policy: OnFailure,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::WriteFile {
                target: TargetPath::Workspace("ro_on_failure.txt"),
                content: "read-only-on-failure",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: Some("command failed; retry without sandbox?"),
            },
            expectation: Expectation::FileCreated {
                target: TargetPath::Workspace("ro_on_failure.txt"),
                content: "read-only-on-failure",
            },
        },
        #[cfg(not(target_os = "linux"))]
        ScenarioSpec {
            name: "read_only_on_failure_escalates_after_sandbox_error_gpt_5_1_no_exit",
            approval_policy: OnFailure,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::WriteFile {
                target: TargetPath::Workspace("ro_on_failure_5_1.txt"),
                content: "read-only-on-failure",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: Some("command failed; retry without sandbox?"),
            },
            expectation: Expectation::FileCreatedNoExitCode {
                target: TargetPath::Workspace("ro_on_failure_5_1.txt"),
                content: "read-only-on-failure",
            },
        },
        ScenarioSpec {
            name: "read_only_on_request_network_escalates_when_approved",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::FetchUrl {
                endpoint: "/ro/network-approved",
                response_body: "read-only-network-ok",
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::NetworkSuccess {
                body_contains: "read-only-network-ok",
            },
        },
        ScenarioSpec {
            name: "read_only_on_request_network_escalates_when_approved_gpt_5_1_no_exit",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::FetchUrl {
                endpoint: "/ro/network-approved",
                response_body: "read-only-network-ok",
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::NetworkSuccessNoExitCode {
                body_contains: "read-only-network-ok",
            },
        },
        ScenarioSpec {
            name: "apply_patch_shell_command_requires_patch_approval",
            approval_policy: UnlessTrusted,
            sandbox_policy: workspace_write(false),
            action: ActionKind::ApplyPatchShell {
                target: TargetPath::Workspace("apply_patch_shell.txt"),
                content: "shell-apply-patch",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: None,
            outcome: Outcome::PatchApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::PatchApplied {
                target: TargetPath::Workspace("apply_patch_shell.txt"),
                content: "shell-apply-patch",
            },
        },
        ScenarioSpec {
            name: "apply_patch_freeform_auto_inside_workspace",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::ApplyPatchFreeform {
                target: TargetPath::Workspace("apply_patch_freeform.txt"),
                content: "freeform-apply-patch",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::Auto,
            expectation: Expectation::PatchApplied {
                target: TargetPath::Workspace("apply_patch_freeform.txt"),
                content: "freeform-apply-patch",
            },
        },
        ScenarioSpec {
            name: "apply_patch_freeform_danger_allows_outside_workspace",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::ApplyPatchFreeform {
                target: TargetPath::OutsideWorkspace("apply_patch_freeform_danger.txt"),
                content: "freeform-patch-danger",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![Feature::ApplyPatchFreeform],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::Auto,
            expectation: Expectation::PatchApplied {
                target: TargetPath::OutsideWorkspace("apply_patch_freeform_danger.txt"),
                content: "freeform-patch-danger",
            },
        },
        ScenarioSpec {
            name: "apply_patch_freeform_outside_requires_patch_approval",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::ApplyPatchFreeform {
                target: TargetPath::OutsideWorkspace("apply_patch_freeform_outside.txt"),
                content: "freeform-patch-outside",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::PatchApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::PatchApplied {
                target: TargetPath::OutsideWorkspace("apply_patch_freeform_outside.txt"),
                content: "freeform-patch-outside",
            },
        },
        ScenarioSpec {
            name: "apply_patch_freeform_outside_denied_blocks_patch",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::ApplyPatchFreeform {
                target: TargetPath::OutsideWorkspace("apply_patch_freeform_outside_denied.txt"),
                content: "freeform-patch-outside-denied",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::PatchApproval {
                decision: ReviewDecision::Denied,
                expected_reason: None,
            },
            expectation: Expectation::FileNotCreated {
                target: TargetPath::OutsideWorkspace("apply_patch_freeform_outside_denied.txt"),
                message_contains: &["patch rejected by user"],
            },
        },
        ScenarioSpec {
            name: "apply_patch_shell_command_outside_requires_patch_approval",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::ApplyPatchShell {
                target: TargetPath::OutsideWorkspace("apply_patch_shell_outside.txt"),
                content: "shell-patch-outside",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: None,
            outcome: Outcome::PatchApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::PatchApplied {
                target: TargetPath::OutsideWorkspace("apply_patch_shell_outside.txt"),
                content: "shell-patch-outside",
            },
        },
        ScenarioSpec {
            name: "apply_patch_freeform_unless_trusted_requires_patch_approval",
            approval_policy: UnlessTrusted,
            sandbox_policy: workspace_write(false),
            action: ActionKind::ApplyPatchFreeform {
                target: TargetPath::Workspace("apply_patch_freeform_unless_trusted.txt"),
                content: "freeform-patch-unless-trusted",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::PatchApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::PatchApplied {
                target: TargetPath::Workspace("apply_patch_freeform_unless_trusted.txt"),
                content: "freeform-patch-unless-trusted",
            },
        },
        ScenarioSpec {
            name: "apply_patch_freeform_never_rejects_outside_workspace",
            approval_policy: Never,
            sandbox_policy: workspace_write(false),
            action: ActionKind::ApplyPatchFreeform {
                target: TargetPath::OutsideWorkspace("apply_patch_freeform_never.txt"),
                content: "freeform-patch-never",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::Auto,
            expectation: Expectation::FileNotCreated {
                target: TargetPath::OutsideWorkspace("apply_patch_freeform_never.txt"),
                message_contains: &[
                    "patch rejected: writing outside of the project; rejected by user approval settings",
                ],
            },
        },
        ScenarioSpec {
            name: "read_only_unless_trusted_requires_approval",
            approval_policy: UnlessTrusted,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::WriteFile {
                target: TargetPath::Workspace("ro_unless_trusted.txt"),
                content: "read-only-unless-trusted",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::FileCreated {
                target: TargetPath::Workspace("ro_unless_trusted.txt"),
                content: "read-only-unless-trusted",
            },
        },
        ScenarioSpec {
            name: "read_only_unless_trusted_requires_approval_gpt_5_1_no_exit",
            approval_policy: UnlessTrusted,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::WriteFile {
                target: TargetPath::Workspace("ro_unless_trusted_5_1.txt"),
                content: "read-only-unless-trusted",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.4"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::FileCreatedNoExitCode {
                target: TargetPath::Workspace("ro_unless_trusted_5_1.txt"),
                content: "read-only-unless-trusted",
            },
        },
        ScenarioSpec {
            name: "read_only_never_reports_sandbox_failure",
            approval_policy: Never,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::WriteFile {
                target: TargetPath::Workspace("ro_never.txt"),
                content: "read-only-never",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: None,
            outcome: Outcome::Auto,
            expectation: Expectation::FileNotCreated {
                target: TargetPath::Workspace("ro_never.txt"),
                message_contains: if cfg!(target_os = "linux") {
                    &["Permission denied|Read-only file system"]
                } else {
                    &[
                        "Permission denied|Operation not permitted|operation not permitted|\
                         Read-only file system",
                    ]
                },
            },
        },
        ScenarioSpec {
            name: "trusted_command_never_runs_without_prompt",
            approval_policy: Never,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::RunCommand {
                command: "echo trusted-never",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::CommandSuccess {
                stdout_contains: "trusted-never",
            },
        },
        ScenarioSpec {
            name: "workspace_write_on_request_allows_workspace_write",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::WriteFile {
                target: TargetPath::Workspace("ww_on_request.txt"),
                content: "workspace-on-request",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::FileCreated {
                target: TargetPath::Workspace("ww_on_request.txt"),
                content: "workspace-on-request",
            },
        },
        ScenarioSpec {
            name: "workspace_write_network_disabled_blocks_network",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::FetchUrl {
                endpoint: "/ww/network-blocked",
                response_body: "workspace-network-blocked",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: None,
            outcome: Outcome::Auto,
            expectation: Expectation::NetworkFailure { expect_tag: "ERR:" },
        },
        ScenarioSpec {
            name: "workspace_write_on_request_requires_approval_outside_workspace",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("ww_on_request_outside.txt"),
                content: "workspace-on-request-outside",
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::FileCreated {
                target: TargetPath::OutsideWorkspace("ww_on_request_outside.txt"),
                content: "workspace-on-request-outside",
            },
        },
        ScenarioSpec {
            name: "workspace_write_network_enabled_allows_network",
            approval_policy: OnRequest,
            sandbox_policy: workspace_write(true),
            action: ActionKind::FetchUrl {
                endpoint: "/ww/network-ok",
                response_body: "workspace-network-ok",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::NetworkSuccess {
                body_contains: "workspace-network-ok",
            },
        },
        #[cfg(not(target_os = "linux"))] // TODO (pakrym): figure out why linux behaves differently
        ScenarioSpec {
            name: "workspace_write_on_failure_escalates_outside_workspace",
            approval_policy: OnFailure,
            sandbox_policy: workspace_write(false),
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("ww_on_failure.txt"),
                content: "workspace-on-failure",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: Some("command failed; retry without sandbox?"),
            },
            expectation: Expectation::FileCreated {
                target: TargetPath::OutsideWorkspace("ww_on_failure.txt"),
                content: "workspace-on-failure",
            },
        },
        ScenarioSpec {
            name: "workspace_write_unless_trusted_requires_approval_outside_workspace",
            approval_policy: UnlessTrusted,
            sandbox_policy: workspace_write(false),
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("ww_unless_trusted.txt"),
                content: "workspace-unless-trusted",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: None,
            },
            expectation: Expectation::FileCreated {
                target: TargetPath::OutsideWorkspace("ww_unless_trusted.txt"),
                content: "workspace-unless-trusted",
            },
        },
        ScenarioSpec {
            name: "workspace_write_never_blocks_outside_workspace",
            approval_policy: Never,
            sandbox_policy: workspace_write(false),
            action: ActionKind::WriteFile {
                target: TargetPath::OutsideWorkspace("ww_never.txt"),
                content: "workspace-never",
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![],
            model_override: None,
            outcome: Outcome::Auto,
            expectation: Expectation::FileNotCreated {
                target: TargetPath::OutsideWorkspace("ww_never.txt"),
                message_contains: if cfg!(target_os = "linux") {
                    &["Permission denied|Read-only file system"]
                } else {
                    &[
                        "Permission denied|Operation not permitted|operation not permitted|\
                         Read-only file system",
                    ]
                },
            },
        },
        ScenarioSpec {
            name: "unified exec on request no approval for safe command",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::RunUnifiedExecCommand {
                command: "echo \"hello unified exec\"",
                justification: None,
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![Feature::UnifiedExec],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::Auto,
            expectation: Expectation::CommandSuccess {
                stdout_contains: "hello unified exec",
            },
        },
        #[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
        // Linux sandbox arg0 test workaround doesn't work on ARM
        ScenarioSpec {
            name: "unified exec on request escalated requires approval",
            approval_policy: OnRequest,
            sandbox_policy: SandboxPolicy::new_read_only_policy(),
            action: ActionKind::RunUnifiedExecCommand {
                command: "python3 -c 'print('\"'\"'escalated unified exec'\"'\"')'",
                justification: Some(DEFAULT_UNIFIED_EXEC_JUSTIFICATION),
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![Feature::UnifiedExec],
            model_override: Some("gpt-5.2"),
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Approved,
                expected_reason: Some(DEFAULT_UNIFIED_EXEC_JUSTIFICATION),
            },
            expectation: Expectation::CommandSuccess {
                stdout_contains: "escalated unified exec",
            },
        },
        ScenarioSpec {
            name: "unified exec on request requires approval unless trusted",
            approval_policy: AskForApproval::UnlessTrusted,
            sandbox_policy: SandboxPolicy::DangerFullAccess,
            action: ActionKind::RunUnifiedExecCommand {
                command: "git reset --hard",
                justification: None,
            },
            sandbox_permissions: SandboxPermissions::UseDefault,
            features: vec![Feature::UnifiedExec],
            model_override: None,
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Denied,
                expected_reason: None,
            },
            expectation: Expectation::CommandFailure {
                output_contains: "rejected by user",
            },
        },
        ScenarioSpec {
            name: "safe command with heredoc and redirect still requires approval",
            approval_policy: AskForApproval::OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::RunUnifiedExecCommand {
                command: "cat <<'EOF' > /tmp/out.txt \nhello\nEOF",
                justification: None,
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![Feature::UnifiedExec],
            model_override: None,
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Denied,
                expected_reason: None,
            },
            expectation: Expectation::CommandFailure {
                output_contains: "rejected by user",
            },
        },
        ScenarioSpec {
            name: "compound command with one safe command still requires approval",
            approval_policy: AskForApproval::OnRequest,
            sandbox_policy: workspace_write(false),
            action: ActionKind::RunUnifiedExecCommand {
                command: "cat ./one.txt && touch ./two.txt",
                justification: None,
            },
            sandbox_permissions: SandboxPermissions::RequireEscalated,
            features: vec![Feature::UnifiedExec],
            model_override: None,
            outcome: Outcome::ExecApproval {
                decision: ReviewDecision::Denied,
                expected_reason: None,
            },
            expectation: Expectation::CommandFailure {
                output_contains: "rejected by user",
            },
        },
    ]
}
