use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolOutput;
use crate::tools::context::ToolPayload;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use codex_protocol::config_types::ModeKind;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::plan_tool::UpdatePlanArgs;
use codex_protocol::protocol::EventMsg;
use serde_json::Value as JsonValue;

pub struct PlanHandler;

pub struct PlanToolOutput {
    message: String,
}

const PLAN_UPDATED_MESSAGE: &str = "Plan updated";
const SELF_REVIEW_CHECKPOINT_MESSAGE: &str = "\
Plan updated

Self-review checkpoint before continuing: actively review the plan as if the user had asked \"review and improve the plan\". Check task order, missing verification, risky assumptions, stale context, user constraints, and user/remote overlap. Revise the plan first if any issue is found.";

impl ToolOutput for PlanToolOutput {
    fn log_preview(&self) -> String {
        self.message.clone()
    }

    fn success_for_logging(&self) -> bool {
        true
    }

    fn to_response_item(&self, call_id: &str, _payload: &ToolPayload) -> ResponseInputItem {
        let mut output = FunctionCallOutputPayload::from_text(self.message.clone());
        output.success = Some(true);

        ResponseInputItem::FunctionCallOutput {
            call_id: call_id.to_string(),
            output,
        }
    }

    fn code_mode_result(&self, _payload: &ToolPayload) -> JsonValue {
        JsonValue::Object(serde_json::Map::new())
    }
}

impl ToolHandler for PlanHandler {
    type Output = PlanToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            call_id,
            payload,
            ..
        } = invocation;

        let arguments = match payload {
            ToolPayload::Function { arguments } => arguments,
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "update_plan handler received unsupported payload".to_string(),
                ));
            }
        };

        let message =
            handle_update_plan(session.as_ref(), turn.as_ref(), arguments, call_id).await?;

        Ok(PlanToolOutput { message })
    }
}

/// This function doesn't do anything useful. However, it gives the model a structured way to record its plan that clients can read and render.
/// So it's the _inputs_ to this function that are useful to clients, not the outputs and neither are actually useful for the model other
/// than forcing it to come up and document a plan (TBD how that affects performance).
pub(crate) async fn handle_update_plan(
    session: &Session,
    turn_context: &TurnContext,
    arguments: String,
    _call_id: String,
) -> Result<String, FunctionCallError> {
    if turn_context.collaboration_mode.mode == ModeKind::Plan {
        return Err(FunctionCallError::RespondToModel(
            "update_plan is a TODO/checklist tool and is not allowed in Plan mode".to_string(),
        ));
    }
    let args = parse_update_plan_arguments(&arguments)?;
    let response = plan_tool_response(&args);
    session
        .send_event(turn_context, EventMsg::PlanUpdate(args))
        .await;
    Ok(response)
}

fn parse_update_plan_arguments(arguments: &str) -> Result<UpdatePlanArgs, FunctionCallError> {
    serde_json::from_str::<UpdatePlanArgs>(arguments).map_err(|e| {
        FunctionCallError::RespondToModel(format!("failed to parse function arguments: {e}"))
    })
}

fn plan_tool_response(args: &UpdatePlanArgs) -> String {
    if is_nontrivial_plan(args) {
        SELF_REVIEW_CHECKPOINT_MESSAGE.to_string()
    } else {
        PLAN_UPDATED_MESSAGE.to_string()
    }
}

fn is_nontrivial_plan(args: &UpdatePlanArgs) -> bool {
    args.plan.len() >= 2
        || args
            .explanation
            .as_deref()
            .is_some_and(|explanation| !explanation.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use codex_protocol::plan_tool::PlanItemArg;
    use codex_protocol::plan_tool::StepStatus;
    use pretty_assertions::assert_eq;

    use super::*;

    fn plan_item(step: &str) -> PlanItemArg {
        PlanItemArg {
            step: step.to_string(),
            status: StepStatus::Pending,
        }
    }

    #[test]
    fn trivial_plan_keeps_compact_output() {
        let args = UpdatePlanArgs {
            explanation: None,
            plan: vec![plan_item("inspect")],
        };

        assert_eq!(plan_tool_response(&args), PLAN_UPDATED_MESSAGE);
    }

    #[test]
    fn nontrivial_plan_includes_self_review_checkpoint() {
        let args = UpdatePlanArgs {
            explanation: None,
            plan: vec![plan_item("inspect"), plan_item("patch")],
        };

        assert_eq!(plan_tool_response(&args), SELF_REVIEW_CHECKPOINT_MESSAGE);
    }

    #[test]
    fn explanation_makes_plan_nontrivial() {
        let args = UpdatePlanArgs {
            explanation: Some("Need to sequence work carefully.".to_string()),
            plan: vec![plan_item("inspect")],
        };

        assert_eq!(plan_tool_response(&args), SELF_REVIEW_CHECKPOINT_MESSAGE);
    }
}
