use super::*;
use pretty_assertions::assert_eq;

#[test]
fn fs_get_metadata_response_round_trips_minimal_fields() {
    let response = FsGetMetadataResponse {
        is_directory: false,
        is_file: true,
        is_symlink: false,
        created_at_ms: 123,
        modified_at_ms: 456,
    };

    let value = serde_json::to_value(&response).expect("serialize fs/getMetadata response");
    assert_eq!(
        value,
        json!({
            "isDirectory": false,
            "isFile": true,
            "isSymlink": false,
            "createdAtMs": 123,
            "modifiedAtMs": 456,
        })
    );

    let decoded = serde_json::from_value::<FsGetMetadataResponse>(value)
        .expect("deserialize fs/getMetadata response");
    assert_eq!(decoded, response);
}

#[test]
fn fs_read_file_response_round_trips_base64_data() {
    let response = FsReadFileResponse {
        data_base64: "aGVsbG8=".to_string(),
    };

    let value = serde_json::to_value(&response).expect("serialize fs/readFile response");
    assert_eq!(
        value,
        json!({
            "dataBase64": "aGVsbG8=",
        })
    );

    let decoded = serde_json::from_value::<FsReadFileResponse>(value)
        .expect("deserialize fs/readFile response");
    assert_eq!(decoded, response);
}

#[test]
fn fs_read_file_params_round_trip() {
    let params = FsReadFileParams {
        path: absolute_path("tmp/example.txt"),
    };

    let value = serde_json::to_value(&params).expect("serialize fs/readFile params");
    assert_eq!(
        value,
        json!({
            "path": absolute_path_string("tmp/example.txt"),
        })
    );

    let decoded =
        serde_json::from_value::<FsReadFileParams>(value).expect("deserialize fs/readFile params");
    assert_eq!(decoded, params);
}

#[test]
fn fs_create_directory_params_round_trip_with_default_recursive() {
    let params = FsCreateDirectoryParams {
        path: absolute_path("tmp/example"),
        recursive: None,
    };

    let value = serde_json::to_value(&params).expect("serialize fs/createDirectory params");
    assert_eq!(
        value,
        json!({
            "path": absolute_path_string("tmp/example"),
            "recursive": null,
        })
    );

    let decoded = serde_json::from_value::<FsCreateDirectoryParams>(value)
        .expect("deserialize fs/createDirectory params");
    assert_eq!(decoded, params);
}

#[test]
fn fs_write_file_params_round_trip_with_base64_data() {
    let params = FsWriteFileParams {
        path: absolute_path("tmp/example.bin"),
        data_base64: "AAE=".to_string(),
    };

    let value = serde_json::to_value(&params).expect("serialize fs/writeFile params");
    assert_eq!(
        value,
        json!({
            "path": absolute_path_string("tmp/example.bin"),
            "dataBase64": "AAE=",
        })
    );

    let decoded = serde_json::from_value::<FsWriteFileParams>(value)
        .expect("deserialize fs/writeFile params");
    assert_eq!(decoded, params);
}

#[test]
fn fs_copy_params_round_trip_with_recursive_directory_copy() {
    let params = FsCopyParams {
        source_path: absolute_path("tmp/source"),
        destination_path: absolute_path("tmp/destination"),
        recursive: true,
    };

    let value = serde_json::to_value(&params).expect("serialize fs/copy params");
    assert_eq!(
        value,
        json!({
            "sourcePath": absolute_path_string("tmp/source"),
            "destinationPath": absolute_path_string("tmp/destination"),
            "recursive": true,
        })
    );

    let decoded =
        serde_json::from_value::<FsCopyParams>(value).expect("deserialize fs/copy params");
    assert_eq!(decoded, params);
}

#[test]
fn thread_shell_command_params_round_trip() {
    let params = ThreadShellCommandParams {
        thread_id: "thr_123".to_string(),
        command: "printf 'hello world\\n'".to_string(),
    };

    let value = serde_json::to_value(&params).expect("serialize thread/shellCommand params");
    assert_eq!(
        value,
        json!({
            "threadId": "thr_123",
            "command": "printf 'hello world\\n'",
        })
    );

    let decoded = serde_json::from_value::<ThreadShellCommandParams>(value)
        .expect("deserialize thread/shellCommand params");
    assert_eq!(decoded, params);
}

#[test]
fn thread_shell_command_response_round_trip() {
    let response = ThreadShellCommandResponse {};

    let value = serde_json::to_value(&response).expect("serialize thread/shellCommand response");
    assert_eq!(value, json!({}));

    let decoded = serde_json::from_value::<ThreadShellCommandResponse>(value)
        .expect("deserialize thread/shellCommand response");
    assert_eq!(decoded, response);
}

#[test]
fn fs_changed_notification_round_trips() {
    let notification = FsChangedNotification {
        watch_id: "0195ec6b-1d6f-7c2e-8c7a-56f2c4a8b9d1".to_string(),
        changed_paths: vec![
            absolute_path("tmp/repo/.git/HEAD"),
            absolute_path("tmp/repo/.git/FETCH_HEAD"),
        ],
    };

    let value = serde_json::to_value(&notification).expect("serialize fs/changed notification");
    assert_eq!(
        value,
        json!({
            "watchId": "0195ec6b-1d6f-7c2e-8c7a-56f2c4a8b9d1",
            "changedPaths": [
                absolute_path_string("tmp/repo/.git/HEAD"),
                absolute_path_string("tmp/repo/.git/FETCH_HEAD"),
            ],
        })
    );

    let decoded = serde_json::from_value::<FsChangedNotification>(value)
        .expect("deserialize fs/changed notification");
    assert_eq!(decoded, notification);
}
