use codex_app_catalog_types as catalog;
use codex_app_server_protocol as protocol;

pub(crate) fn app_infos_to_v2(values: Vec<catalog::AppInfo>) -> Vec<protocol::AppInfo> {
    values.into_iter().map(app_info_to_v2).collect()
}

pub(crate) fn app_info_to_v2(value: catalog::AppInfo) -> protocol::AppInfo {
    protocol::AppInfo {
        id: value.id,
        name: value.name,
        description: value.description,
        logo_url: value.logo_url,
        logo_url_dark: value.logo_url_dark,
        distribution_channel: value.distribution_channel,
        branding: value.branding.map(app_branding_to_v2),
        app_metadata: value.app_metadata.map(app_metadata_to_v2),
        labels: value.labels,
        install_url: value.install_url,
        is_accessible: value.is_accessible,
        is_enabled: value.is_enabled,
        plugin_display_names: value.plugin_display_names,
    }
}

pub(crate) fn app_summary_from_catalog(
    value: catalog::AppInfo,
    needs_auth: bool,
) -> protocol::AppSummary {
    protocol::AppSummary {
        id: value.id,
        name: value.name,
        description: value.description,
        install_url: value.install_url,
        needs_auth,
    }
}

fn app_branding_to_v2(value: catalog::AppBranding) -> protocol::AppBranding {
    protocol::AppBranding {
        category: value.category,
        developer: value.developer,
        website: value.website,
        privacy_policy: value.privacy_policy,
        terms_of_service: value.terms_of_service,
        is_discoverable_app: value.is_discoverable_app,
    }
}

fn app_metadata_to_v2(value: catalog::AppMetadata) -> protocol::AppMetadata {
    protocol::AppMetadata {
        review: value.review.map(app_review_to_v2),
        categories: value.categories,
        sub_categories: value.sub_categories,
        seo_description: value.seo_description,
        screenshots: value
            .screenshots
            .map(|screenshots| screenshots.into_iter().map(app_screenshot_to_v2).collect()),
        developer: value.developer,
        version: value.version,
        version_id: value.version_id,
        version_notes: value.version_notes,
        first_party_type: value.first_party_type,
        first_party_requires_install: value.first_party_requires_install,
        show_in_composer_when_unlinked: value.show_in_composer_when_unlinked,
    }
}

fn app_review_to_v2(value: catalog::AppReview) -> protocol::AppReview {
    protocol::AppReview {
        status: value.status,
    }
}

fn app_screenshot_to_v2(value: catalog::AppScreenshot) -> protocol::AppScreenshot {
    protocol::AppScreenshot {
        url: value.url,
        file_id: value.file_id,
        user_prompt: value.user_prompt,
    }
}
