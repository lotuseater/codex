use anyhow::Result;
use codex_core::config::AutoCoordinatorMode;
use codex_core::config::Config;
use codex_features::Feature;
// fork-local: MultiAgentMode is used by the fork's multi-agent-mode tests (upstream dropped this import)
use codex_protocol::config_types::MultiAgentMode;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::MULTI_AGENT_MODE_OPEN_TAG;
use codex_protocol::protocol::MultiAgentVersion;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::ThreadSettingsOverrides;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::mount_sse_once_match;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use core_test_support::wait_for_event_with_timeout;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::time::Duration;

const NO_SPAWN_TEXT: &str = "Do not spawn sub-agents unless the user or applicable AGENTS.md/skill instructions explicitly ask for sub-agents, delegation, or parallel agent work.";
const PROACTIVE_TEXT: &str = "Proactive multi-agent delegation is active.";
const CUSTOM_MODE_HINT_TEXT: &str = "Use the configured delegation policy.";
// fork-local: emitted when multi-agent mode is inactive; used by the fork's multi-agent-mode tests
const NO_MODE_TEXT: &str = "Multi-agent delegation mode instructions are inactive.";

fn add_ultra_reasoning(model_info: &mut ModelInfo) {
    model_info.supports_reasoning_summaries = true;
    model_info
        .supported_reasoning_levels
        .push(ReasoningEffortPreset {
            effort: ReasoningEffort::Ultra,
            description: "Ultra".to_string(),
        });
}

fn configure_multi_agent_v2(config: &mut Config) {
    config
        .features
        .enable(Feature::MultiAgentV2)
        .expect("test config should allow feature update");
}

// Configuring a custom mode hint also enables multi-agent V2 for the test.
fn configure_custom_mode_hint(config: &mut Config) {
    configure_multi_agent_v2(config);
    config.multi_agent_v2.multi_agent_mode_hint_text = Some(CUSTOM_MODE_HINT_TEXT.to_string());
}

fn configure_ultra(config: &mut Config) {
    configure_multi_agent_v2(config);
    config.model_reasoning_effort = Some(ReasoningEffort::Ultra);
}

fn developer_texts(input: &[Value]) -> Vec<&str> {
    input
        .iter()
        .filter(|item| item.get("role").and_then(Value::as_str) == Some("developer"))
        .filter_map(|item| item.get("content")?.as_array())
        .flatten()
        .filter_map(|content| content.get("text")?.as_str())
        .collect()
}

fn count_containing(texts: &[&str], target: &str) -> usize {
    texts.iter().filter(|text| text.contains(target)).count()
}

fn user_texts(input: &[Value]) -> Vec<&str> {
    input
        .iter()
        .filter(|item| item.get("role").and_then(Value::as_str) == Some("user"))
        .filter_map(|item| item.get("content")?.as_array())
        .flatten()
        .filter_map(|content| content.get("text")?.as_str())
        .collect()
}

async fn submit_turn(
    codex: &codex_core::CodexThread,
    prompt: &str,
    effort: Option<ReasoningEffort>,
) -> Result<()> {
    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: prompt.to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides {
                effort: effort.map(Some),
                ..Default::default()
            },
        })
        .await?;
    wait_for_event(codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;
    Ok(())
}

// fork-local: submits a turn that carries a multi-agent-mode override. Upstream refactored
// `submit_turn` to carry a reasoning-effort override instead; the fork's multi-agent-mode
// tests need this mode-carrying variant.
async fn submit_turn_mode(
    codex: &codex_core::CodexThread,
    prompt: &str,
    mode: Option<MultiAgentMode>,
) -> Result<()> {
    codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: prompt.to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides {
                multi_agent_mode: mode,
                ..Default::default()
            },
        })
        .await?;
    wait_for_event(codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ultra_reasoning_uses_max_and_proactive_mode() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let test = test_codex()
        .with_model_info_override("gpt-5.4", add_ultra_reasoning)
        .with_config(configure_ultra)
        .build(&server)
        .await?;

    submit_turn(&test.codex, "hello", /*effort*/ None).await?;

    let request = response.single_request();
    assert_eq!(
        request.body_json()["reasoning"]["effort"].as_str(),
        Some("max")
    );
    let input = request.input();
    let texts = developer_texts(&input);
    assert_eq!(
        (
            count_containing(&texts, NO_SPAWN_TEXT),
            count_containing(&texts, PROACTIVE_TEXT),
        ),
        (0, 1)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn configured_mode_hint_uses_custom_mode_across_reasoning_efforts() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_sequence(
        &server,
        (1..=2)
            .map(|index| {
                sse(vec![
                    ev_response_created(&format!("resp-{index}")),
                    ev_completed(&format!("resp-{index}")),
                ])
            })
            .collect(),
    )
    .await;
    let test = test_codex()
        .with_model_info_override("gpt-5.4", add_ultra_reasoning)
        .with_config(configure_custom_mode_hint)
        .build(&server)
        .await?;
    let rollout_path = test
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");

    submit_turn(&test.codex, "explicit", Some(ReasoningEffort::High)).await?;
    submit_turn(&test.codex, "proactive", Some(ReasoningEffort::Ultra)).await?;

    let requests = responses.requests();
    let first_input = requests[0].input();
    let first_texts = developer_texts(&first_input);
    let second_input = requests[1].input();
    let second_texts = developer_texts(&second_input);
    let instruction_counts = |texts: &[&str]| {
        (
            count_containing(texts, CUSTOM_MODE_HINT_TEXT),
            count_containing(texts, NO_SPAWN_TEXT),
            count_containing(texts, PROACTIVE_TEXT),
        )
    };
    assert_eq!(instruction_counts(&first_texts), (1, 0, 0));
    assert_eq!(instruction_counts(&second_texts), (1, 0, 0));
    let rollout_values = std::fs::read_to_string(rollout_path)?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<serde_json::Result<Vec<_>>>()?;
    let recorded_modes = rollout_values
        .iter()
        .filter(|value| value.get("type").and_then(Value::as_str) == Some("turn_context"))
        .filter_map(|value| value.pointer("/payload/multi_agent_mode").cloned())
        .collect::<Vec<_>>();
    assert_eq!(
        recorded_modes,
        [
            json!({"custom": CUSTOM_MODE_HINT_TEXT}),
            json!({"custom": CUSTOM_MODE_HINT_TEXT}),
        ]
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_configured_mode_hint_suppresses_builtin_text() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let test = test_codex()
        .with_config(|config| {
            configure_multi_agent_v2(config);
            config.multi_agent_v2.multi_agent_mode_hint_text = Some(String::new());
        })
        .build(&server)
        .await?;

    submit_turn(&test.codex, "hello", Some(ReasoningEffort::High)).await?;

    let input = response.single_request().input();
    let texts = developer_texts(&input);
    assert_eq!(
        (
            count_containing(&texts, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&texts, NO_SPAWN_TEXT),
            count_containing(&texts, PROACTIVE_TEXT),
        ),
        (1, 0, 0)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaving_ultra_after_cold_resume_emits_explicit_mode() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_sequence(
        &server,
        (1..=2)
            .map(|index| {
                sse(vec![
                    ev_response_created(&format!("resp-{index}")),
                    ev_completed(&format!("resp-{index}")),
                ])
            })
            .collect(),
    )
    .await;
    let initial = test_codex()
        .with_model_info_override("gpt-5.4", add_ultra_reasoning)
        .with_config(configure_ultra)
        .build(&server)
        .await?;
    let home = initial.home.clone();
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");

    submit_turn(&initial.codex, "before resume", /*effort*/ None).await?;
    drop(initial);

    let mut resume_builder = test_codex()
        .with_model_info_override("gpt-5.4", add_ultra_reasoning)
        .with_config(configure_ultra);
    let resumed = resume_builder.resume(&server, home, rollout_path).await?;
    submit_turn(&resumed.codex, "after resume", Some(ReasoningEffort::High)).await?;

    let requests = responses.requests();
    assert_eq!(
        (
            requests[0].body_json()["reasoning"]["effort"]
                .as_str()
                .map(str::to_string),
            requests[1].body_json()["reasoning"]["effort"]
                .as_str()
                .map(str::to_string),
        ),
        (Some("max".to_string()), Some("high".to_string()))
    );
    let resumed_input = requests[1].input();
    let texts = developer_texts(&resumed_input);
    assert_eq!(
        (
            count_containing(&texts, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&texts, NO_SPAWN_TEXT),
            count_containing(&texts, PROACTIVE_TEXT),
        ),
        (2, 1, 1)
    );

    Ok(())
}

// fork-local: multi-agent-mode feature tests. Upstream's auto-merge replaced these with its
// own reasoning-effort tests in the non-conflicted region; restored here to preserve fork
// coverage. `MultiAgentMode::None` was refactored to `Custom(String)` upstream (wire `none` =>
// `Custom("")`), so the mechanical migration `None -> Custom(String::new())` is applied. Runtime
// assertions (instruction-text counts) may need reconciliation against upstream's reworded text.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_agent_mode_is_sticky_and_emits_only_on_change() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_sequence(
        &server,
        (1..=5)
            .map(|index| {
                sse(vec![
                    ev_response_created(&format!("resp-{index}")),
                    ev_completed(&format!("resp-{index}")),
                ])
            })
            .collect(),
    )
    .await;
    let test = test_codex()
        .with_config(|config| {
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("test config should allow feature update");
            // The fresh-root auto-coordinator flip (codex_handle.rs) would start
            // this session Proactive under the default AutoCoordinatorMode::Auto;
            // pin it Off so this test keeps exercising the sticky/emit-on-change
            // mechanics from the upstream default (ExplicitRequestOnly).
            config.multi_agent_v2.auto_coordinator = AutoCoordinatorMode::Off;
        })
        .build(&server)
        .await?;

    submit_turn_mode(&test.codex, "turn one", /*mode*/ None).await?;
    assert_eq!(
        test.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::ExplicitRequestOnly
    );
    submit_turn_mode(&test.codex, "turn two", Some(MultiAgentMode::Proactive)).await?;
    submit_turn_mode(&test.codex, "turn three", /*mode*/ None).await?;
    submit_turn_mode(
        &test.codex,
        "turn four",
        Some(MultiAgentMode::Custom(String::new())),
    )
    .await?;
    submit_turn_mode(&test.codex, "turn five", /*mode*/ None).await?;

    assert_eq!(
        test.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::Custom(String::new())
    );

    let requests = responses.requests();
    let inputs = requests
        .iter()
        .map(core_test_support::responses::ResponsesRequest::input)
        .collect::<Vec<_>>();
    let first = developer_texts(&inputs[0]);
    let second = developer_texts(&inputs[1]);
    let third = developer_texts(&inputs[2]);
    let fourth = developer_texts(&inputs[3]);
    let fifth = developer_texts(&inputs[4]);

    assert_eq!(
        (
            count_containing(&first, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&first, NO_SPAWN_TEXT),
            count_containing(&first, PROACTIVE_TEXT),
        ),
        (1, 1, 0)
    );
    assert_eq!(
        (
            count_containing(&second, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&second, NO_SPAWN_TEXT),
            count_containing(&second, PROACTIVE_TEXT),
        ),
        (2, 1, 1)
    );
    assert_eq!(
        (
            count_containing(&third, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&third, NO_SPAWN_TEXT),
            count_containing(&third, PROACTIVE_TEXT),
        ),
        (2, 1, 1)
    );
    assert_eq!(
        (
            count_containing(&fourth, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&fourth, NO_SPAWN_TEXT),
            count_containing(&fourth, PROACTIVE_TEXT),
            count_containing(&fourth, NO_MODE_TEXT),
        ),
        (3, 1, 1, 1)
    );
    assert_eq!(
        (
            count_containing(&fifth, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&fifth, NO_SPAWN_TEXT),
            count_containing(&fifth, PROACTIVE_TEXT),
            count_containing(&fifth, NO_MODE_TEXT),
        ),
        (3, 1, 1, 1)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_agent_mode_none_omits_instructions_and_survives_resume() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_sequence(
        &server,
        (1..=2)
            .map(|index| {
                sse(vec![
                    ev_response_created(&format!("resp-{index}")),
                    ev_completed(&format!("resp-{index}")),
                ])
            })
            .collect(),
    )
    .await;
    let initial = test_codex()
        .with_config(|config| {
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await?;
    let home = initial.home.clone();
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");

    submit_turn_mode(
        &initial.codex,
        "before resume",
        Some(MultiAgentMode::Custom(String::new())),
    )
    .await?;
    assert_eq!(
        initial.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::Custom(String::new())
    );
    drop(initial);

    let mut resume_builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::MultiAgentV2)
            .expect("test config should allow feature update");
    });
    let resumed = resume_builder.resume(&server, home, rollout_path).await?;
    submit_turn_mode(&resumed.codex, "after resume", /*mode*/ None).await?;

    assert_eq!(
        resumed.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::Custom(String::new())
    );
    let requests = responses.requests();
    assert_eq!(requests.len(), 2);
    for request in requests {
        let input = request.input();
        let texts = developer_texts(&input);
        assert_eq!(
            (
                count_containing(&texts, MULTI_AGENT_MODE_OPEN_TAG),
                count_containing(&texts, NO_SPAWN_TEXT),
                count_containing(&texts, PROACTIVE_TEXT),
                count_containing(&texts, NO_MODE_TEXT),
            ),
            (0, 0, 0, 0)
        );
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_agent_mode_applies_without_usage_hint_text() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let test = test_codex()
        .with_config(|config| {
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("test config should allow feature update");
            config.multi_agent_v2.root_agent_usage_hint_text = None;
        })
        .build(&server)
        .await?;

    submit_turn_mode(&test.codex, "hello", Some(MultiAgentMode::Proactive)).await?;

    let input = responses.single_request().input();
    let texts = developer_texts(&input);
    assert_eq!(
        (
            count_containing(&texts, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&texts, PROACTIVE_TEXT),
        ),
        (1, 1)
    );

    Ok(())
}

// fork-local: an explicit per-thread multi-agent-mode override (set via
// `SessionSettingsUpdate::multi_agent_mode`) must win over the effort/
// auto-coordinator-derived default that `effective_multi_agent_mode` would
// otherwise produce.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_multi_agent_mode_override_is_honored() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let _response = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    // Fresh root under multi-agent V2 with auto-coordination ON and no usage-hint
    // text or Ultra effort: `effective_multi_agent_mode` derives `Proactive`
    // (the root + `auto_coordinator_active()` branch).
    let test = test_codex()
        .with_config(|config| {
            configure_multi_agent_v2(config);
            config.multi_agent_v2.root_agent_usage_hint_text = None;
            config.multi_agent_v2.auto_coordinator = AutoCoordinatorMode::Always;
        })
        .build(&server)
        .await?;

    // Baseline (no override): the derivation drives the effective mode to
    // `Proactive`, so the override target below genuinely differs from it.
    assert_eq!(
        test.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::Proactive,
        "a fresh auto-coordinator root derives Proactive when no override is set"
    );

    // Explicitly pin the thread to `ExplicitRequestOnly`; the override must be
    // honored verbatim instead of the `Proactive` derivation.
    submit_turn_mode(
        &test.codex,
        "pin explicit",
        Some(MultiAgentMode::ExplicitRequestOnly),
    )
    .await?;

    assert_eq!(
        test.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::ExplicitRequestOnly,
        "an explicit multi_agent_mode override is honored over the derived default"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_compares_against_previous_effective_multi_agent_mode() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_sequence(
        &server,
        (1..=2)
            .map(|index| {
                sse(vec![
                    ev_response_created(&format!("resp-{index}")),
                    ev_completed(&format!("resp-{index}")),
                ])
            })
            .collect(),
    )
    .await;
    let initial = test_codex()
        .with_config(|config| {
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await?;
    let home = initial.home.clone();
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");

    submit_turn_mode(
        &initial.codex,
        "before resume",
        Some(MultiAgentMode::Proactive),
    )
    .await?;
    drop(initial);

    let mut resume_builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::MultiAgentV2)
            .expect("test config should allow feature update");
    });
    let resumed = resume_builder.resume(&server, home, rollout_path).await?;
    submit_turn_mode(&resumed.codex, "after resume", /*mode*/ None).await?;

    assert_eq!(
        resumed.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::Proactive
    );

    let requests = responses.requests();
    let resumed_input = requests[1].input();
    let texts = developer_texts(&resumed_input);
    assert_eq!(
        (
            count_containing(&texts, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&texts, NO_SPAWN_TEXT),
            count_containing(&texts, PROACTIVE_TEXT),
        ),
        (1, 0, 1)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_upgrades_multi_agent_version_to_v2_when_config_enables_v2() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let _responses = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    // Initial session runs under multi-agent V1 (Collab enabled, V2 disabled), so
    // its persisted history resolves to V1 on resume.
    let initial = test_codex()
        .with_config(|config| {
            config
                .features
                .enable(Feature::Collab)
                .expect("test config should allow feature update");
        })
        .build(&server)
        .await?;
    let home = initial.home.clone();
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");
    submit_turn(&initial.codex, "before resume", /*mode*/ None).await?;
    drop(initial);

    // Resume under a config that enables multi-agent V2. The restored/defaulted V1
    // session must be upgraded to V2 so it honors the current V2 tool surface;
    // without the upgrade the pre-set OnceLock would lock the resumed session to V1.
    let mut resume_builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::MultiAgentV2)
            .expect("test config should allow feature update");
    });
    let resumed = resume_builder.resume(&server, home, rollout_path).await?;

    assert_eq!(
        resumed.codex.multi_agent_version(),
        Some(MultiAgentVersion::V2)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resume_flips_root_explicit_request_only_to_proactive_when_auto_coordinator_active()
-> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let _responses = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    // Initial root session: multi-agent V2 with auto-coordination OFF, so the fresh
    // root starts (and persists) `ExplicitRequestOnly`.
    let initial = test_codex()
        .with_config(|config| {
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("test config should allow feature update");
            config.multi_agent_v2.auto_coordinator = AutoCoordinatorMode::Off;
        })
        .build(&server)
        .await?;
    assert_eq!(
        initial.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::ExplicitRequestOnly
    );
    let home = initial.home.clone();
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");
    submit_turn(&initial.codex, "before resume", /*mode*/ None).await?;
    drop(initial);

    // Resume the ROOT with auto-coordination enabled: the restored
    // `ExplicitRequestOnly` is promoted to `Proactive` so the resumed root may
    // delegate without an explicit request.
    let mut resume_builder = test_codex().with_config(|config| {
        config
            .features
            .enable(Feature::MultiAgentV2)
            .expect("test config should allow feature update");
        config.multi_agent_v2.auto_coordinator = AutoCoordinatorMode::Always;
    });
    let resumed = resume_builder.resume(&server, home, rollout_path).await?;

    assert_eq!(
        resumed.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::Proactive
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_agent_mode_is_retained_without_multi_agent_v2() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let test = test_codex().build(&server).await?;

    submit_turn_mode(&test.codex, "hello", Some(MultiAgentMode::Proactive)).await?;

    assert_eq!(
        test.codex.config_snapshot().await.multi_agent_mode,
        MultiAgentMode::Proactive
    );
    let input = responses.single_request().input();
    let texts = developer_texts(&input);
    assert_eq!(
        (
            count_containing(&texts, MULTI_AGENT_MODE_OPEN_TAG),
            count_containing(&texts, PROACTIVE_TEXT),
        ),
        (0, 0)
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ultra_on_multi_agent_v1_uses_max_without_mode_instructions() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let test = test_codex()
        .with_model_info_override("gpt-5.4", add_ultra_reasoning)
        .with_config(|config| {
            config.model_reasoning_effort = Some(ReasoningEffort::Ultra);
        })
        .build(&server)
        .await?;

    submit_turn(&test.codex, "hello", /*effort*/ None).await?;

    let request = response.single_request();
    assert_eq!(
        request.body_json()["reasoning"]["effort"].as_str(),
        Some("max")
    );
    let input = request.input();
    let texts = developer_texts(&input);
    assert_eq!(count_containing(&texts, MULTI_AGENT_MODE_OPEN_TAG), 0);

    Ok(())
}

fn request_body_contains(request: &wiremock::Request, text: &str) -> bool {
    serde_json::from_slice::<Value>(&request.body).is_ok_and(|body| body.to_string().contains(text))
}

fn request_has_function_call_output(request: &wiremock::Request, call_id: &str) -> bool {
    serde_json::from_slice::<Value>(&request.body).is_ok_and(|body| {
        body.get("input")
            .and_then(Value::as_array)
            .is_some_and(|items| {
                items.iter().any(|item| {
                    item.get("type").and_then(Value::as_str) == Some("function_call_output")
                        && item.get("call_id").and_then(Value::as_str) == Some(call_id)
                })
            })
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn usage_hint_rides_user_channel_when_role_user() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const HINT_TEXT: &str = "Custom delegation hint: prefer parallel workers.";
    const FRAGMENT_OPEN_TAG: &str = "<codex_internal_context source=\"multi_agent_usage_hint\">";

    let server = start_mock_server().await;
    let responses = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let test = test_codex()
        .with_config(|config| {
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("test config should allow feature update");
            // `delegation_injection_role` defaults to `user`; keep the
            // auto-coordinator off so the usage hint is the only delegation
            // nudge in flight.
            config.multi_agent_v2.root_agent_usage_hint_text = Some(HINT_TEXT.to_string());
            config.multi_agent_v2.auto_coordinator = AutoCoordinatorMode::Off;
        })
        .build(&server)
        .await?;

    submit_turn(&test.codex, "hello", None).await?;

    let input = responses.single_request().input();
    assert_eq!(
        count_containing(&developer_texts(&input), HINT_TEXT),
        0,
        "usage hint must not ride the discounted developer channel"
    );
    let texts = user_texts(&input);
    let hint_texts: Vec<&&str> = texts
        .iter()
        .filter(|text| text.contains(HINT_TEXT))
        .collect();
    assert_eq!(
        hint_texts.len(),
        1,
        "usage hint must reach the model exactly once as user-role text"
    );
    assert!(
        hint_texts[0].contains(FRAGMENT_OPEN_TAG),
        "user-channel usage hint must stay wrapped in its internal-context fragment"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_drain_receives_auto_coordinator_framing_once() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const WAIT_CALL_ID: &str = "wait-call";
    const STEER_PROMPT: &str = "steered mid-turn task";
    const FRAMING_SNIPPET: &str = "You are operating as a COORDINATOR";
    // 1x1 transparent PNG. An image-only initial turn carries no user text, so
    // the fresh-input fusion site stays inert and the pending-input drain is
    // the only site that can fuse the auto-coordinator framing.
    const IMAGE_DATA_URL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

    let server = start_mock_server().await;
    let request_log = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    WAIT_CALL_ID,
                    "wait_agent",
                    // Generous tool deadline: a working steer-wake returns
                    // immediately; only genuine wake failures run this out.
                    r#"{"timeout_ms":60000}"#,
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("m-2", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;
    let test = test_codex()
        .with_model("gpt-5.4")
        .with_config(|config| {
            config
                .features
                .enable(Feature::Collab)
                .expect("test config should allow feature update");
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("test config should allow feature update");
            config.multi_agent_v2.auto_coordinator = AutoCoordinatorMode::Always;
            // Silence the initial-context usage hint so the framing fused on
            // the drained input is the only delegation text in the exchange.
            config.multi_agent_v2.usage_hint_enabled = false;
        })
        .build(&server)
        .await?;

    test.codex
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Image {
                image_url: IMAGE_DATA_URL.to_string(),
                detail: None,
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides::default(),
        })
        .await?;
    // The image-only initial turn must reach the wait_agent park before we
    // steer the human follow-up; a park here (not a pass) is the real failure.
    wait_for_event_with_timeout(
        &test.codex,
        |event| matches!(event, EventMsg::CollabWaitingBegin(_)),
        Duration::from_secs(30),
    )
    .await;

    test.codex
        .steer_input(
            vec![UserInput::Text {
                text: STEER_PROMPT.to_string(),
                text_elements: Vec::new(),
            }],
            /*additional_context*/ Default::default(),
            /*expected_turn_id*/ None,
            /*client_user_message_id*/ None,
            /*responsesapi_client_metadata*/ None,
        )
        // SteerInputError does not implement std::error::Error, so `?` under
        // anyhow will not compile; expect() mirrors suite/pending_input.rs.
        .await
        .expect("steer user input");
    // Exceeds the tool's 60s deadline so a park here crisply signals a real
    // steer-wake failure rather than scheduling skew under gate load.
    wait_for_event_with_timeout(
        &test.codex,
        |event| matches!(event, EventMsg::TurnComplete(_)),
        Duration::from_secs(70),
    )
    .await;

    let requests = request_log.requests();
    assert_eq!(
        requests.len(),
        2,
        "expected the wait turn and the drained follow-up request"
    );
    assert_eq!(
        count_containing(&user_texts(&requests[0].input()), FRAMING_SNIPPET),
        0,
        "an image-only initial turn has no user text, so nothing fuses up front"
    );
    let follow_up_input = requests[1].input();
    let steered_message = follow_up_input
        .iter()
        .find(|item| {
            item.get("role").and_then(Value::as_str) == Some("user")
                && item
                    .get("content")
                    .and_then(Value::as_array)
                    .is_some_and(|content| {
                        content.iter().any(|block| {
                            block.get("text").and_then(Value::as_str) == Some(STEER_PROMPT)
                        })
                    })
        })
        .expect("steered user message should ride the follow-up request");
    let steered_texts: Vec<&str> = steered_message
        .get("content")
        .and_then(Value::as_array)
        .expect("steered message content")
        .iter()
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect();
    assert_eq!(
        count_containing(&steered_texts, FRAMING_SNIPPET),
        1,
        "the drained pending input must receive the auto-coordinator framing"
    );
    assert_eq!(
        count_containing(&user_texts(&follow_up_input), FRAMING_SNIPPET),
        1,
        "the framing is bounded to one fused copy per run_turn"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forked_child_history_strips_generated_hints_and_framing() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const SPAWN_CALL_ID: &str = "spawn-fork-call";
    const ROOT_PROMPT: &str = "coordinate the big refactor";
    const CHILD_TASK: &str = "handle the parser module slice";
    const ROOT_HINT_SNIPPET: &str = "MultiAgentV2 planning mode is enabled.";
    const SUBAGENT_HINT_SNIPPET: &str = "MultiAgentV2 worker mode is enabled.";
    const FRAMING_SNIPPET: &str = "You are operating as a COORDINATOR";

    let server = start_mock_server().await;
    let spawn_args = json!({
        "message": CHILD_TASK,
        "task_name": "parser_slice",
        "fork_turns": "all",
    })
    .to_string();
    let root_first = mount_sse_once_match(
        &server,
        |request: &wiremock::Request| {
            request_body_contains(request, ROOT_PROMPT)
                && !request_body_contains(request, CHILD_TASK)
        },
        sse(vec![
            ev_response_created("resp-root-1"),
            ev_function_call(SPAWN_CALL_ID, "spawn_agent", &spawn_args),
            ev_completed("resp-root-1"),
        ]),
    )
    .await;
    let root_follow_up = mount_sse_once_match(
        &server,
        |request: &wiremock::Request| request_has_function_call_output(request, SPAWN_CALL_ID),
        sse(vec![
            ev_response_created("resp-root-2"),
            ev_assistant_message("m-root-2", "spawned the worker"),
            ev_completed("resp-root-2"),
        ]),
    )
    .await;
    let child_requests = mount_sse_once_match(
        &server,
        |request: &wiremock::Request| {
            request_body_contains(request, CHILD_TASK)
                && !request_has_function_call_output(request, SPAWN_CALL_ID)
        },
        sse(vec![
            ev_response_created("resp-child-1"),
            ev_assistant_message("m-child-1", "child done"),
            ev_completed("resp-child-1"),
        ]),
    )
    .await;

    let test = test_codex()
        .with_model("gpt-5.4")
        .with_config(|config| {
            config
                .features
                .enable(Feature::Collab)
                .expect("test config should allow feature update");
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("test config should allow feature update");
            config.multi_agent_v2.auto_coordinator = AutoCoordinatorMode::Always;
            config.multi_agent_v2.max_concurrent_threads_per_session = 2;
        })
        .build(&server)
        .await?;

    submit_turn(&test.codex, ROOT_PROMPT, None).await?;

    // Positive control on the parent: the fused framing rode the root turn, so
    // the absence assertions on the child below are non-vacuous.
    let root_input = root_first.single_request().input();
    assert!(
        user_texts(&root_input)
            .iter()
            .any(|text| text.contains(FRAMING_SNIPPET)),
        "auto-coordinator framing must ride the root turn"
    );

    // The spawn round-trip must have SUCCEEDED before polling for the child; a
    // "collab spawn failed: {reason}" output otherwise makes this timeout mute.
    let follow_up_input = root_follow_up.single_request().input();
    let spawn_output = follow_up_input
        .iter()
        .find(|item| {
            item.get("type").and_then(Value::as_str) == Some("function_call_output")
                && item.get("call_id").and_then(Value::as_str) == Some(SPAWN_CALL_ID)
        })
        .and_then(|item| item.get("output"))
        .and_then(Value::as_str)
        .expect("root follow-up must carry the spawn_agent function_call_output");
    assert!(
        !spawn_output.starts_with("collab spawn failed:"),
        "spawn_agent must succeed before the child can run, got: {spawn_output}"
    );

    let child_request = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(request) = child_requests.requests().into_iter().next() {
                return request;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("forked child should issue its first model request");

    let child_input = child_request.input();
    let child_texts = user_texts(&child_input);
    assert!(
        child_texts.iter().any(|text| text.contains(ROOT_PROMPT)),
        "the fork must carry the parent's real user message"
    );
    assert!(
        child_texts
            .iter()
            .any(|text| text.contains(SUBAGENT_HINT_SNIPPET)),
        "the forked child must receive the worker-mode usage hint"
    );
    assert_eq!(
        child_texts
            .iter()
            .filter(|text| text.contains(ROOT_HINT_SNIPPET))
            .count(),
        0,
        "the parent's generated root usage hint must be stripped from forked history"
    );
    let forked_root_message = child_input
        .iter()
        .find(|item| {
            item.get("role").and_then(Value::as_str) == Some("user")
                && item
                    .get("content")
                    .and_then(Value::as_array)
                    .is_some_and(|content| {
                        content.iter().any(|block| {
                            block
                                .get("text")
                                .and_then(Value::as_str)
                                .is_some_and(|text| text.contains(ROOT_PROMPT))
                        })
                    })
        })
        .expect("forked child history should retain the parent's user message");
    let forked_root_texts: Vec<&str> = forked_root_message
        .get("content")
        .and_then(Value::as_array)
        .expect("forked user message content")
        .iter()
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect();
    assert_eq!(
        count_containing(&forked_root_texts, FRAMING_SNIPPET),
        0,
        "the framing fused onto the parent's user message must be stripped in the fork"
    );

    Ok(())
}
