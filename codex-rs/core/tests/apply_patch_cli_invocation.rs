#![allow(clippy::expect_used)]

mod support;

#[path = "suite/apply_patch_harness.rs"]
mod apply_patch_harness;

use anyhow::Result;
use apply_patch_harness::apply_patch_harness;
use apply_patch_harness::apply_patch_harness_with;
use apply_patch_harness::mount_apply_patch;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use codex_core_test_runtime::assert_regex_match;
use codex_core_test_runtime::responses::ev_apply_patch_custom_tool_call;
use codex_core_test_runtime::responses::ev_assistant_message;
use codex_core_test_runtime::responses::ev_completed;
use codex_core_test_runtime::responses::ev_response_created;
use codex_core_test_runtime::responses::ev_shell_command_call;
use codex_core_test_runtime::responses::ev_shell_command_call_with_args;
use codex_core_test_runtime::responses::mount_sse_sequence;
use codex_core_test_runtime::responses::sse;
use codex_core_test_runtime::skip_if_no_network;
use codex_core_test_runtime::skip_if_remote;
use codex_core_test_runtime::test_codex::ApplyPatchModelOutput;
#[cfg(target_os = "linux")]
use codex_sandboxing::landlock::CODEX_LINUX_SANDBOX_ARG0;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use test_case::test_case;
use wiremock::Mock;
use wiremock::Respond;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path_regex;

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_cli_uses_codex_self_exe_with_linux_sandbox_helper_alias() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;
    let codex_linux_sandbox_exe = harness
        .test()
        .config
        .codex_linux_sandbox_exe
        .as_ref()
        .expect("linux test config should include codex-linux-sandbox helper");
    assert_eq!(
        codex_linux_sandbox_exe
            .file_name()
            .and_then(|name| name.to_str()),
        Some(CODEX_LINUX_SANDBOX_ARG0),
    );

    let patch = "*** Begin Patch\n*** Add File: helper-alias.txt\n+hello\n*** End Patch";
    let call_id = "apply-helper-alias";
    mount_apply_patch(
        &harness,
        call_id,
        patch,
        "done",
        ApplyPatchModelOutput::Freeform,
    )
    .await;

    harness.submit("please apply helper alias patch").await?;

    let out = harness
        .apply_patch_output(call_id, ApplyPatchModelOutput::Freeform)
        .await;
    assert_regex_match(
        r"(?s)^Exit code: 0.*Success\. Updated the following files:\nA helper-alias\.txt\n?$",
        &out,
    );
    assert_eq!(harness.read_file_text("helper-alias.txt").await?, "hello\n");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_shell_command_heredoc_with_cd_updates_relative_workdir() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness_with(|builder| builder.with_model("gpt-5.4")).await?;

    // Prepare a file inside a subdir; update it via cd && apply_patch heredoc form.
    harness.write_file("sub/in_sub.txt", "before\n").await?;

    let script = "cd sub && apply_patch <<'EOF'\n*** Begin Patch\n*** Update File: in_sub.txt\n@@\n-before\n+after\n*** End Patch\nEOF\n";
    let call_id = "shell-heredoc-cd";
    let bodies = vec![
        sse(vec![
            ev_response_created("resp-1"),
            ev_shell_command_call(call_id, script),
            ev_completed("resp-1"),
        ]),
        sse(vec![
            ev_assistant_message("msg-1", "ok"),
            ev_completed("resp-2"),
        ]),
    ];
    mount_sse_sequence(harness.server(), bodies).await;

    harness.submit("apply via shell heredoc with cd").await?;

    let out = harness.function_call_stdout(call_id).await;
    assert!(
        out.contains("Success."),
        "expected successful apply_patch invocation via shell_command: {out}"
    );
    assert_eq!(harness.read_file_text("sub/in_sub.txt").await?, "after\n");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn apply_patch_cli_can_use_shell_command_output_as_patch_input() -> Result<()> {
    skip_if_no_network!(Ok(()));
    skip_if_remote!(
        Ok(()),
        "shell_command output producer runs in the test runner, not in the remote apply_patch workspace",
    );

    let harness =
        apply_patch_harness_with(|builder| builder.with_model("gpt-5.4").with_windows_cmd_shell())
            .await?;

    let source_contents = "line1\nnaïve café\nline3\n";
    harness.write_file("source.txt", source_contents).await?;

    let read_call_id = "read-source";
    let apply_call_id = "apply-from-read";

    fn stdout_from_shell_output(output: &str) -> String {
        let normalized = output.replace("\r\n", "\n").replace('\r', "\n");
        normalized
            .split_once("Output:\n")
            .map(|x| x.1)
            .unwrap_or("")
            .trim_end_matches('\n')
            .to_string()
    }

    fn function_call_output_text(body: &serde_json::Value, call_id: &str) -> String {
        body.get("input")
            .and_then(serde_json::Value::as_array)
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("type").and_then(serde_json::Value::as_str)
                        == Some("function_call_output")
                        && item.get("call_id").and_then(serde_json::Value::as_str) == Some(call_id)
                })
            })
            .and_then(|item| item.get("output").and_then(serde_json::Value::as_str))
            .expect("function_call_output output string")
            .to_string()
    }

    struct DynamicApplyFromRead {
        num_calls: AtomicI32,
        read_call_id: String,
        apply_call_id: String,
    }

    impl Respond for DynamicApplyFromRead {
        fn respond(&self, request: &wiremock::Request) -> ResponseTemplate {
            let call_num = self.num_calls.fetch_add(1, Ordering::SeqCst);
            match call_num {
                0 => {
                    let command = if cfg!(windows) {
                        // Encode the nested PowerShell script so `cmd.exe /c` does not leave the
                        // read command wrapped in quotes, and suppress progress records so the
                        // shell tool only returns the file contents back to apply_patch.
                        let script = "$ProgressPreference = 'SilentlyContinue'; [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); [System.IO.File]::ReadAllText('source.txt', [System.Text.UTF8Encoding]::new($false))";
                        let encoded = BASE64_STANDARD.encode(
                            script
                                .encode_utf16()
                                .flat_map(u16::to_le_bytes)
                                .collect::<Vec<u8>>(),
                        );
                        format!(
                            "powershell.exe -NoLogo -NoProfile -NonInteractive -EncodedCommand {encoded}"
                        )
                    } else {
                        "cat source.txt".to_string()
                    };
                    let args = json!({
                        "command": command,
                        "login": false,
                    });
                    let body = sse(vec![
                        ev_response_created("resp-1"),
                        ev_shell_command_call_with_args(&self.read_call_id, &args),
                        ev_completed("resp-1"),
                    ]);
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(body)
                }
                1 => {
                    let body_json: serde_json::Value =
                        request.body_json().expect("request body should be json");
                    let read_output = function_call_output_text(&body_json, &self.read_call_id);
                    let stdout = stdout_from_shell_output(&read_output);
                    let patch_lines = stdout
                        .lines()
                        .map(|line| format!("+{line}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let patch = format!(
                        "*** Begin Patch\n*** Add File: target.txt\n{patch_lines}\n*** End Patch"
                    );

                    let body = sse(vec![
                        ev_response_created("resp-2"),
                        ev_apply_patch_custom_tool_call(&self.apply_call_id, &patch),
                        ev_completed("resp-2"),
                    ]);
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(body)
                }
                2 => {
                    let body = sse(vec![
                        ev_assistant_message("msg-1", "ok"),
                        ev_completed("resp-3"),
                    ]);
                    ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(body)
                }
                _ => panic!("no response for call {call_num}"),
            }
        }
    }

    let responder = DynamicApplyFromRead {
        num_calls: AtomicI32::new(0),
        read_call_id: read_call_id.to_string(),
        apply_call_id: apply_call_id.to_string(),
    };
    Mock::given(method("POST"))
        .and(path_regex(".*/responses$"))
        .respond_with(responder)
        .expect(3)
        .mount(harness.server())
        .await;

    harness
        .submit("read source.txt, then apply it to target.txt")
        .await?;

    let target_contents = harness.read_file_text("target.txt").await?;
    assert_eq!(target_contents, source_contents);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_case(ApplyPatchModelOutput::ShellCommandViaHeredoc)]
async fn apply_patch_shell_accepts_lenient_heredoc_wrapped_patch(
    model_output: ApplyPatchModelOutput,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let harness = apply_patch_harness().await?;

    let file_name = "lenient.txt";
    let patch_inner =
        format!("*** Begin Patch\n*** Add File: {file_name}\n+lenient\n*** End Patch\n");
    let call_id = "apply-lenient";
    mount_apply_patch(&harness, call_id, patch_inner.as_str(), "ok", model_output).await;

    harness.submit("apply lenient heredoc patch").await?;

    assert_eq!(harness.read_file_text(file_name).await?, "lenient\n");
    Ok(())
}
