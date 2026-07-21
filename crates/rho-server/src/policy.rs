use rho_protocol::ClientKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    Inspect,
    Summarize,
    Visualize,
    ModifyObject,
    OverwriteFile,
    InstallPackage,
    ShellExecution,
    UploadData,
}

impl PolicyAction {
    pub fn from_workspace_request(request_type: &str) -> Option<Self> {
        match request_type {
            "workspace.snapshot" | "workspace.inspect_object" => Some(Self::Inspect),
            "workspace.execute" => Some(Self::ModifyObject),
            "workspace.render_document" | "workspace.set_project_root" => Some(Self::OverwriteFile),
            _ => None,
        }
    }

    pub fn is_read_only(self) -> bool {
        matches!(self, Self::Inspect | Self::Summarize | Self::Visualize)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyPrincipal {
    DirectUser,
    McpHost,
    InternalAgentAct,
    InternalAgentReadOnly,
    System,
}

impl PolicyPrincipal {
    pub fn from_client_kind(kind: ClientKind) -> Self {
        match kind {
            ClientKind::Mcp => Self::McpHost,
            ClientKind::Agent => Self::InternalAgentAct,
            ClientKind::Cli | ClientKind::Web | ClientKind::Desktop => Self::DirectUser,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Automatic,
    ExplicitUserIntent,
    DelegatedHostApproval,
    RequireBrokerApproval,
    Deny,
}

impl PolicyDecision {
    pub fn permits_execution(self) -> bool {
        matches!(
            self,
            Self::Automatic | Self::ExplicitUserIntent | Self::DelegatedHostApproval
        )
    }

    pub fn policy_name(self) -> &'static str {
        match self {
            Self::Automatic => "rho.automatic_read_only",
            Self::ExplicitUserIntent => "rho.explicit_user_intent",
            Self::DelegatedHostApproval => "rho.mcp_host_human_in_loop",
            Self::RequireBrokerApproval => "rho.broker_approval_required",
            Self::Deny => "rho.read_only_mode_denied",
        }
    }
}

pub struct PolicyEngine;

impl PolicyEngine {
    pub fn evaluate(principal: PolicyPrincipal, action: PolicyAction) -> PolicyDecision {
        if action.is_read_only() {
            return PolicyDecision::Automatic;
        }
        match principal {
            PolicyPrincipal::DirectUser => PolicyDecision::ExplicitUserIntent,
            PolicyPrincipal::McpHost => PolicyDecision::DelegatedHostApproval,
            PolicyPrincipal::InternalAgentAct => PolicyDecision::RequireBrokerApproval,
            PolicyPrincipal::InternalAgentReadOnly => PolicyDecision::Deny,
            PolicyPrincipal::System => PolicyDecision::Automatic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_scientific_actions_are_automatic_for_every_boundary() {
        for principal in [
            PolicyPrincipal::DirectUser,
            PolicyPrincipal::McpHost,
            PolicyPrincipal::InternalAgentAct,
            PolicyPrincipal::InternalAgentReadOnly,
            PolicyPrincipal::System,
        ] {
            for action in [
                PolicyAction::Inspect,
                PolicyAction::Summarize,
                PolicyAction::Visualize,
            ] {
                assert_eq!(
                    PolicyEngine::evaluate(principal, action),
                    PolicyDecision::Automatic
                );
            }
        }
    }

    #[test]
    fn consequential_actions_share_one_boundary_aware_decision_table() {
        for action in [
            PolicyAction::ModifyObject,
            PolicyAction::OverwriteFile,
            PolicyAction::InstallPackage,
            PolicyAction::ShellExecution,
            PolicyAction::UploadData,
        ] {
            assert_eq!(
                PolicyEngine::evaluate(PolicyPrincipal::DirectUser, action),
                PolicyDecision::ExplicitUserIntent
            );
            assert_eq!(
                PolicyEngine::evaluate(PolicyPrincipal::McpHost, action),
                PolicyDecision::DelegatedHostApproval
            );
            assert_eq!(
                PolicyEngine::evaluate(PolicyPrincipal::InternalAgentAct, action),
                PolicyDecision::RequireBrokerApproval
            );
            assert_eq!(
                PolicyEngine::evaluate(PolicyPrincipal::InternalAgentReadOnly, action),
                PolicyDecision::Deny
            );
        }
    }
}
