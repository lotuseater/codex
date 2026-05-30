//! Implements the `codex features` subcommand dispatch and its helpers.
//!
//! The dispatch and the feature-config helpers are moved verbatim from `main.rs`.
//! Behavior, validation order, and output are unchanged.

use codex_config::edit::ConfigEditsBuilder;
use codex_core::config::ConfigBuilder;
use codex_core::config::find_codex_home;
use codex_features::FEATURES;
use codex_features::Stage;
use codex_tui::Cli as TuiCli;
use codex_utils_cli::CliConfigOverrides;

use crate::FeatureSetArgs;
use crate::FeatureToggles;
use crate::FeaturesCli;
use crate::FeaturesSubcommand;
use crate::reject_remote_mode_for_subcommand;

pub async fn run_features(
    features_cli: FeaturesCli,
    root_config_overrides: CliConfigOverrides,
    interactive: &TuiCli,
    root_remote: Option<&str>,
    root_remote_auth_token_env: Option<&str>,
) -> anyhow::Result<()> {
    let FeaturesCli { sub } = features_cli;
    match sub {
        FeaturesSubcommand::List => {
            reject_remote_mode_for_subcommand(
                root_remote,
                root_remote_auth_token_env,
                "features list",
            )?;
            let mut cli_kv_overrides = root_config_overrides
                .parse_overrides()
                .map_err(anyhow::Error::msg)?;

            // Honor `--search` via the canonical web_search mode.
            if interactive.web_search {
                cli_kv_overrides.push((
                    "web_search".to_string(),
                    toml::Value::String("live".to_string()),
                ));
            }

            let config = ConfigBuilder::default()
                .cli_overrides(cli_kv_overrides)
                .build()
                .await?;
            let mut rows = Vec::with_capacity(FEATURES.len());
            let mut name_width = 0;
            let mut stage_width = 0;
            for def in FEATURES {
                let name = def.key;
                let stage = stage_str(def.stage);
                let enabled = config.features.enabled(def.id);
                name_width = name_width.max(name.len());
                stage_width = stage_width.max(stage.len());
                rows.push((name, stage, enabled));
            }
            rows.sort_unstable_by_key(|(name, _, _)| *name);

            for (name, stage, enabled) in rows {
                println!("{name:<name_width$}  {stage:<stage_width$}  {enabled}");
            }
        }
        FeaturesSubcommand::Enable(FeatureSetArgs { feature }) => {
            reject_remote_mode_for_subcommand(
                root_remote,
                root_remote_auth_token_env,
                "features enable",
            )?;
            enable_feature_in_config(&feature).await?;
        }
        FeaturesSubcommand::Disable(FeatureSetArgs { feature }) => {
            reject_remote_mode_for_subcommand(
                root_remote,
                root_remote_auth_token_env,
                "features disable",
            )?;
            disable_feature_in_config(&feature).await?;
        }
    }

    Ok(())
}

async fn enable_feature_in_config(feature: &str) -> anyhow::Result<()> {
    FeatureToggles::validate_feature(feature)?;
    let codex_home = find_codex_home()?;
    ConfigEditsBuilder::new(&codex_home)
        .set_feature_enabled(feature, /*enabled*/ true)
        .apply()
        .await?;
    println!("Enabled feature `{feature}` in config.toml.");
    maybe_print_under_development_feature_warning(&codex_home, feature);
    Ok(())
}

async fn disable_feature_in_config(feature: &str) -> anyhow::Result<()> {
    FeatureToggles::validate_feature(feature)?;
    let codex_home = find_codex_home()?;
    ConfigEditsBuilder::new(&codex_home)
        .set_feature_enabled(feature, /*enabled*/ false)
        .apply()
        .await?;
    println!("Disabled feature `{feature}` in config.toml.");
    Ok(())
}

fn maybe_print_under_development_feature_warning(codex_home: &std::path::Path, feature: &str) {
    let Some(spec) = FEATURES.iter().find(|spec| spec.key == feature) else {
        return;
    };
    if !matches!(spec.stage, Stage::UnderDevelopment) {
        return;
    }

    let config_path = codex_home.join(codex_config::CONFIG_TOML_FILE);
    eprintln!(
        "Under-development features enabled: {feature}. Under-development features are incomplete and may behave unpredictably. To suppress this warning, set `suppress_unstable_features_warning = true` in {}.",
        config_path.display()
    );
}

fn stage_str(stage: Stage) -> &'static str {
    match stage {
        Stage::UnderDevelopment => "under development",
        Stage::Experimental { .. } => "experimental",
        Stage::Stable => "stable",
        Stage::Deprecated => "deprecated",
        Stage::Removed => "removed",
    }
}
