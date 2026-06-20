use super::common::make_config_for_test;
use super::*;
use pretty_assertions::assert_eq;

async fn write_linked_worktree_pointer(
    repo_root: &Path,
    worktree_root: &Path,
) -> std::io::Result<()> {
    let worktree_git_dir = repo_root.join(".git/worktrees/feature-x");
    tokio::fs::create_dir_all(&worktree_git_dir).await?;
    tokio::fs::write(
        worktree_root.join(".git"),
        format!("gitdir: {}\n", worktree_git_dir.display()),
    )
    .await
}

async fn write_project_hook_config(
    dot_codex_folder: &Path,
    foo: Option<&str>,
    command: &str,
) -> std::io::Result<()> {
    tokio::fs::create_dir_all(dot_codex_folder).await?;
    let foo = foo
        .map(|value| format!("foo = \"{value}\"\n\n"))
        .unwrap_or_default();
    tokio::fs::write(
        dot_codex_folder.join(CONFIG_TOML_FILE),
        format!(
            r#"{foo}[hooks]

[[hooks.PreToolUse]]
matcher = "Bash"

[[hooks.PreToolUse.hooks]]
type = "command"
command = "{command}"
"#
        ),
    )
    .await
}

#[tokio::test]
async fn linked_worktree_project_layers_keep_worktree_config_but_use_root_repo_hooks()
-> std::io::Result<()> {
    let tmp = tempdir()?;
    let repo_root = tmp.path().join("repo");
    let repo_child = repo_root.join("child");
    let worktree_root = tmp.path().join("worktree");
    let worktree_child = worktree_root.join("child");

    tokio::fs::create_dir_all(worktree_root.join(".codex")).await?;
    tokio::fs::create_dir_all(worktree_child.join(".codex")).await?;
    write_linked_worktree_pointer(&repo_root, &worktree_root).await?;
    write_project_hook_config(
        &repo_root.join(".codex"),
        Some("repo-root"),
        "echo repo root hook",
    )
    .await?;
    write_project_hook_config(
        &repo_child.join(".codex"),
        Some("repo-child"),
        "echo repo child hook",
    )
    .await?;
    write_project_hook_config(
        &worktree_root.join(".codex"),
        Some("worktree-root"),
        "echo worktree root hook",
    )
    .await?;
    write_project_hook_config(
        &worktree_child.join(".codex"),
        Some("worktree-child"),
        "echo worktree child hook",
    )
    .await?;

    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    make_config_for_test(
        &codex_home,
        &repo_root,
        TrustLevel::Trusted,
        /*project_root_markers*/ None,
    )
    .await?;

    let cwd = AbsolutePathBuf::from_absolute_path(&worktree_child)?;
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home,
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    let project_layers: Vec<_> = layers
        .layers_high_to_low()
        .into_iter()
        .filter(|layer| matches!(layer.name, ConfigLayerSource::Project { .. }))
        .collect();
    assert_eq!(project_layers.len(), 2);
    assert_eq!(
        project_layers[0].hooks_config_folder(),
        Some(AbsolutePathBuf::from_absolute_path(
            repo_child.join(".codex")
        )?)
    );
    assert_eq!(
        project_layers[1].hooks_config_folder(),
        Some(AbsolutePathBuf::from_absolute_path(
            repo_root.join(".codex")
        )?)
    );
    assert_eq!(
        project_layers[0]
            .config
            .get("foo")
            .and_then(TomlValue::as_str),
        Some("worktree-child")
    );
    assert_eq!(
        project_hook_command(project_layers[0]),
        Some("echo repo child hook")
    );
    assert_eq!(
        project_layers[1]
            .config
            .get("foo")
            .and_then(TomlValue::as_str),
        Some("worktree-root")
    );
    assert_eq!(
        project_hook_command(project_layers[1]),
        Some("echo repo root hook")
    );

    Ok(())
}

#[tokio::test]
async fn linked_worktree_project_layers_use_root_repo_hooks_without_worktree_config_toml()
-> std::io::Result<()> {
    let tmp = tempdir()?;
    let repo_root = tmp.path().join("repo");
    let worktree_root = tmp.path().join("worktree");

    tokio::fs::create_dir_all(worktree_root.join(".codex")).await?;
    write_linked_worktree_pointer(&repo_root, &worktree_root).await?;
    write_project_hook_config(
        &repo_root.join(".codex"),
        /*foo*/ None,
        "echo repo root hook",
    )
    .await?;

    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    make_config_for_test(
        &codex_home,
        &repo_root,
        TrustLevel::Trusted,
        /*project_root_markers*/ None,
    )
    .await?;

    let cwd = AbsolutePathBuf::from_absolute_path(&worktree_root)?;
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home,
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    let project_layers: Vec<_> = layers
        .layers_high_to_low()
        .into_iter()
        .filter(|layer| matches!(layer.name, ConfigLayerSource::Project { .. }))
        .collect();
    assert_eq!(project_layers.len(), 1);
    assert_eq!(
        project_layers[0].hooks_config_folder(),
        Some(AbsolutePathBuf::from_absolute_path(
            repo_root.join(".codex")
        )?)
    );
    assert_eq!(
        project_hook_command(project_layers[0]),
        Some("echo repo root hook")
    );

    Ok(())
}

#[tokio::test]
async fn nested_project_root_markers_do_not_redirect_regular_repo_hooks() -> std::io::Result<()> {
    let tmp = tempdir()?;
    let repo_root = tmp.path().join("repo");
    let project_root = repo_root.join("project");
    let nested = project_root.join("child");

    tokio::fs::create_dir_all(repo_root.join(".git")).await?;
    tokio::fs::create_dir_all(&project_root).await?;
    tokio::fs::write(project_root.join(".hg"), "hg").await?;
    write_project_hook_config(
        &repo_root.join(".codex"),
        /*foo*/ None,
        "echo repo root hook",
    )
    .await?;
    write_project_hook_config(
        &project_root.join(".codex"),
        /*foo*/ None,
        "echo project root hook",
    )
    .await?;
    write_project_hook_config(
        &nested.join(".codex"),
        /*foo*/ None,
        "echo nested hook",
    )
    .await?;

    let codex_home = tmp.path().join("home");
    tokio::fs::create_dir_all(&codex_home).await?;
    make_config_for_test(
        &codex_home,
        &project_root,
        TrustLevel::Trusted,
        Some(vec![".hg".to_string()]),
    )
    .await?;

    let cwd = AbsolutePathBuf::from_absolute_path(&nested)?;
    let layers = load_config_layers_state(
        LOCAL_FS.as_ref(),
        &codex_home,
        Some(cwd),
        &[] as &[(String, TomlValue)],
        LoaderOverrides::default(),
        &codex_config::NoopThreadConfigLoader,
    )
    .await?;

    let project_layers: Vec<_> = layers
        .layers_high_to_low()
        .into_iter()
        .filter(|layer| matches!(layer.name, ConfigLayerSource::Project { .. }))
        .collect();
    assert_eq!(project_layers.len(), 2);
    assert_eq!(
        project_layers[0].hooks_config_folder(),
        Some(AbsolutePathBuf::from_absolute_path(nested.join(".codex"))?)
    );
    assert_eq!(
        project_layers[1].hooks_config_folder(),
        Some(AbsolutePathBuf::from_absolute_path(
            project_root.join(".codex")
        )?)
    );
    assert_eq!(
        project_hook_command(project_layers[0]),
        Some("echo nested hook")
    );
    assert_eq!(
        project_hook_command(project_layers[1]),
        Some("echo project root hook")
    );

    Ok(())
}

fn project_hook_command(layer: &ConfigLayerEntry) -> Option<&str> {
    layer
        .config
        .get("hooks")?
        .get("PreToolUse")?
        .as_array()?
        .first()?
        .get("hooks")?
        .as_array()?
        .first()?
        .get("command")?
        .as_str()
}
