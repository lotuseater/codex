use crate::AskForApproval;

const PROMPT_CONFLICT_REASON: &str =
    "approval required by policy, but AskForApproval is set to Never";
const REJECT_SANDBOX_APPROVAL_REASON: &str =
    "approval required by policy, but AskForApproval::Granular.sandbox_approval is false";
const REJECT_RULES_APPROVAL_REASON: &str =
    "approval required by policy rule, but AskForApproval::Granular.rules is false";

/// Approval surfaces controlled by `AskForApproval`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApprovalGate {
    SandboxApproval,
    PolicyRule,
    SkillApproval,
    RequestPermissions,
    McpElicitation,
}

/// Why an `AskForApproval` policy rejected an approval surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApprovalPolicyRejection {
    ApprovalPolicyNever,
    SandboxApprovalDisabled,
    RulesApprovalDisabled,
    SkillApprovalDisabled,
    RequestPermissionsDisabled,
    McpElicitationsDisabled,
}

impl ApprovalPolicyRejection {
    /// Legacy user-facing reason for exec-policy approval prompts.
    ///
    /// Other rejection kinds intentionally return `None` until their call sites
    /// expose a stable message.
    pub const fn exec_prompt_reason(self) -> Option<&'static str> {
        match self {
            Self::ApprovalPolicyNever => Some(PROMPT_CONFLICT_REASON),
            Self::SandboxApprovalDisabled => Some(REJECT_SANDBOX_APPROVAL_REASON),
            Self::RulesApprovalDisabled => Some(REJECT_RULES_APPROVAL_REASON),
            Self::SkillApprovalDisabled
            | Self::RequestPermissionsDisabled
            | Self::McpElicitationsDisabled => None,
        }
    }
}

impl AskForApproval {
    /// Returns the policy rejection for an approval surface, or `None` when the
    /// prompt may be surfaced.
    pub const fn rejection_for_gate(self, gate: ApprovalGate) -> Option<ApprovalPolicyRejection> {
        match self {
            Self::Never => Some(ApprovalPolicyRejection::ApprovalPolicyNever),
            Self::OnFailure | Self::OnRequest | Self::UnlessTrusted => None,
            Self::Granular(granular_config) => match gate {
                ApprovalGate::SandboxApproval if !granular_config.allows_sandbox_approval() => {
                    Some(ApprovalPolicyRejection::SandboxApprovalDisabled)
                }
                ApprovalGate::PolicyRule if !granular_config.allows_rules_approval() => {
                    Some(ApprovalPolicyRejection::RulesApprovalDisabled)
                }
                ApprovalGate::SkillApproval if !granular_config.allows_skill_approval() => {
                    Some(ApprovalPolicyRejection::SkillApprovalDisabled)
                }
                ApprovalGate::RequestPermissions
                    if !granular_config.allows_request_permissions() =>
                {
                    Some(ApprovalPolicyRejection::RequestPermissionsDisabled)
                }
                ApprovalGate::McpElicitation if !granular_config.allows_mcp_elicitations() => {
                    Some(ApprovalPolicyRejection::McpElicitationsDisabled)
                }
                ApprovalGate::SandboxApproval
                | ApprovalGate::PolicyRule
                | ApprovalGate::SkillApproval
                | ApprovalGate::RequestPermissions
                | ApprovalGate::McpElicitation => None,
            },
        }
    }

    /// Whether an approval surface may be surfaced under this policy.
    pub const fn allows_approval_gate(self, gate: ApprovalGate) -> bool {
        match self.rejection_for_gate(gate) {
            Some(_) => false,
            None => true,
        }
    }

    /// Returns the legacy exec prompt rejection reason for an approval surface.
    pub const fn exec_prompt_rejection_reason(self, gate: ApprovalGate) -> Option<&'static str> {
        match self.rejection_for_gate(gate) {
            Some(rejection) => rejection.exec_prompt_reason(),
            None => None,
        }
    }

    /// Whether this approval policy is eligible for automatic guardian review.
    pub const fn allows_auto_review_routing(self) -> bool {
        match self {
            Self::OnRequest | Self::Granular(_) => true,
            Self::UnlessTrusted | Self::OnFailure | Self::Never => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GranularApprovalConfig;
    use pretty_assertions::assert_eq;

    #[test]
    fn broad_approval_modes_allow_all_gates() {
        for policy in [
            AskForApproval::OnFailure,
            AskForApproval::OnRequest,
            AskForApproval::UnlessTrusted,
        ] {
            for gate in [
                ApprovalGate::Sandbox,
                ApprovalGate::PolicyRule,
                ApprovalGate::SkillApproval,
                ApprovalGate::RequestPermissions,
                ApprovalGate::McpElicitation,
            ] {
                assert_eq!(policy.rejection_for_gate(gate), None);
                assert!(policy.allows_approval_gate(gate));
            }
        }
    }

    #[test]
    fn never_rejects_all_gates() {
        for gate in [
                ApprovalGate::Sandbox,
                ApprovalGate::PolicyRule,
                ApprovalGate::SkillApproval,
                ApprovalGate::RequestPermissions,
                ApprovalGate::McpElicitation,
            ] {
            assert_eq!(
                AskForApproval::Never.rejection_for_gate(gate),
                Some(ApprovalPolicyRejection::ApprovalPolicyNever)
            );
            assert!(!AskForApproval::Never.allows_approval_gate(gate));
        }
    }

    #[test]
    fn granular_gates_are_independent() {
        let policy = AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: false,
            rules: true,
            skill_approval: false,
            request_permissions: true,
            mcp_elicitations: false,
        });

        assert_eq!(
            policy.rejection_for_gate(ApprovalGate::SandboxApproval),
            Some(ApprovalPolicyRejection::SandboxApprovalDisabled)
        );
        assert_eq!(policy.rejection_for_gate(ApprovalGate::PolicyRule), None);
        assert_eq!(
            policy.rejection_for_gate(ApprovalGate::SkillApproval),
            Some(ApprovalPolicyRejection::SkillApprovalDisabled)
        );
        assert_eq!(
            policy.rejection_for_gate(ApprovalGate::RequestPermissions),
            None
        );
        assert_eq!(
            policy.rejection_for_gate(ApprovalGate::McpElicitation),
            Some(ApprovalPolicyRejection::McpElicitationsDisabled)
        );
    }

    #[test]
    fn auto_review_routing_is_limited_to_on_request_and_granular_policies() {
        assert!(AskForApproval::OnRequest.allows_auto_review_routing());
        assert!(
            AskForApproval::Granular(GranularApprovalConfig {
                sandbox_approval: true,
                rules: true,
                skill_approval: true,
                request_permissions: true,
                mcp_elicitations: true,
            })
            .allows_auto_review_routing()
        );
        assert!(!AskForApproval::UnlessTrusted.allows_auto_review_routing());
        assert!(!AskForApproval::OnFailure.allows_auto_review_routing());
        assert!(!AskForApproval::Never.allows_auto_review_routing());
    }

    #[test]
    fn exec_prompt_rejection_reason_preserves_current_messages() {
        let sandbox_disabled = AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: false,
            rules: true,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        });
        let rules_disabled = AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: false,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        });

        assert_eq!(
            AskForApproval::Never.exec_prompt_rejection_reason(ApprovalGate::SandboxApproval),
            Some("approval required by policy, but AskForApproval is set to Never")
        );
        assert_eq!(
            sandbox_disabled.exec_prompt_rejection_reason(ApprovalGate::SandboxApproval),
            Some(
                "approval required by policy, but AskForApproval::Granular.sandbox_approval is false"
            )
        );
        assert_eq!(
            rules_disabled.exec_prompt_rejection_reason(ApprovalGate::PolicyRule),
            Some(
                "approval required by policy rule, but AskForApproval::Granular.rules is false"
            )
        );
        assert_eq!(
            rules_disabled.exec_prompt_rejection_reason(ApprovalGate::RequestPermissions),
            None
        );
    }
}
