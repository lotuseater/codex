#![cfg(not(target_os = "windows"))]

#[path = "view_image_common.rs"]
mod common;

use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn view_image_routes_to_selected_remote_environment() -> anyhow::Result<()> {
    skip_if_no_network!(Ok(()));
    let Some(_remote_env) = get_remote_test_env() else {
        return Ok(());
    };

    let server = start_mock_server().await;
    let mut builder = test_codex();
    let test = builder.build_remote_aware(&server).await?;
    let local_cwd = TempDir::new()?;
    fs::write(local_cwd.path().join("remote.png"), b"not a remote image")?;
    let local_selection = TurnEnvironmentSelection {
        environment_id: LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: local_cwd.path().abs(),
    };
    let remote_cwd = PathBuf::from(format!(
        "/tmp/codex-view-image-routing-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis()
    ))
    .abs();
    let image_path = remote_cwd.join("remote.png");
    test.fs()
        .create_directory(
            &remote_cwd,
            CreateDirectoryOptions { recursive: true },
            /*sandbox*/ None,
        )
        .await?;
    let png = BASE64_STANDARD.decode(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=",
    )?;
    test.fs()
        .write_file(&image_path, png, /*sandbox*/ None)
        .await?;
    let remote_selection = TurnEnvironmentSelection {
        environment_id: REMOTE_ENVIRONMENT_ID.to_string(),
        cwd: remote_cwd.clone(),
    };
    let call_id = "call-view-image-multi-env";
    let response_mock = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("resp-1"),
                ev_function_call(
                    call_id,
                    "view_image",
                    &json!({
                        "path": "remote.png",
                        "environment_id": REMOTE_ENVIRONMENT_ID,
                    })
                    .to_string(),
                ),
                ev_completed("resp-1"),
            ]),
            sse(vec![
                ev_response_created("resp-2"),
                ev_assistant_message("msg-1", "done"),
                ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    test.submit_turn_with_environments(
        "route view image",
        Some(vec![local_selection, remote_selection]),
    )
    .await?;

    let output = response_mock
        .last_request()
        .context("missing request containing view_image output")?
        .function_call_output(call_id)
        .clone();
    let output_items = output
        .get("output")
        .and_then(Value::as_array)
        .context("view_image output should be content items")?;
    assert_eq!(output_items.len(), 1);
    let image_url = output_items[0]
        .get("image_url")
        .and_then(Value::as_str)
        .context("view_image output should include image_url")?;
    assert!(
        image_url.starts_with("data:image/png;base64,"),
        "unexpected image_url: {image_url}",
    );

    test.fs()
        .remove(
            &remote_cwd,
            RemoveOptions {
                recursive: true,
                force: true,
            },
            /*sandbox*/ None,
        )
        .await?;

    Ok(())
}
