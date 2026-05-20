mod support;

#[path = "suite/code_mode.rs"]
mod code_mode;
#[path = "suite/deprecation_notice.rs"]
mod deprecation_notice;
#[path = "suite/image_rollout.rs"]
mod image_rollout;
#[path = "suite/model_overrides.rs"]
mod model_overrides;
#[path = "suite/model_switching.rs"]
mod model_switching;
#[path = "suite/model_visible_layout.rs"]
mod model_visible_layout;
#[path = "suite/models_cache_ttl.rs"]
mod models_cache_ttl;
#[path = "suite/models_etag_responses.rs"]
mod models_etag_responses;
#[path = "suite/override_updates.rs"]
mod override_updates;
#[path = "suite/personality.rs"]
mod personality;
#[path = "suite/personality_migration.rs"]
mod personality_migration;
#[path = "suite/prompt_caching.rs"]
mod prompt_caching;
#[path = "suite/prompt_debug_tests.rs"]
mod prompt_debug_tests;
#[path = "suite/quota_exceeded.rs"]
mod quota_exceeded;
#[path = "suite/remote_models.rs"]
mod remote_models;
#[path = "suite/rollout_list_find.rs"]
mod rollout_list_find;
#[path = "suite/safety_check_downgrade.rs"]
mod safety_check_downgrade;
#[path = "suite/unstable_features_warning.rs"]
mod unstable_features_warning;
