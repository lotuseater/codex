use crate::status::RateLimitSnapshotDisplay;
use chrono::DateTime;
use chrono::Duration as ChronoDuration;
use chrono::Local;
use codex_protocol::protocol::TokenUsageInfo;
use ratatui::prelude::Stylize;
use ratatui::text::Line;

pub(crate) fn line(
    token_info: Option<&TokenUsageInfo>,
    context_window: Option<i64>,
    codex_rate_limit: Option<&RateLimitSnapshotDisplay>,
    now: DateTime<Local>,
) -> Option<Line<'static>> {
    let mut parts = Vec::new();
    if let Some(token_percent) = token_used_percent(token_info, context_window) {
        parts.push(format!("{token_percent}% tokens"));
    }
    if let Some(reset_percent) = reset_elapsed_percent(codex_rate_limit, now) {
        parts.push(format!("{reset_percent}% reset"));
    }
    if parts.is_empty() {
        None
    } else {
        Some(Line::from(parts.join(" · ")).dim())
    }
}

fn token_used_percent(
    token_info: Option<&TokenUsageInfo>,
    context_window: Option<i64>,
) -> Option<i64> {
    let context_window = context_window?;
    let usage = &token_info?.last_token_usage;
    Some((100 - usage.percent_of_context_window_remaining(context_window)).clamp(0, 100))
}

fn reset_elapsed_percent(
    codex_rate_limit: Option<&RateLimitSnapshotDisplay>,
    now: DateTime<Local>,
) -> Option<i64> {
    let codex_rate_limit = codex_rate_limit?;
    let window = [
        codex_rate_limit.primary.as_ref(),
        codex_rate_limit.secondary.as_ref(),
    ]
    .into_iter()
    .flatten()
    .find(|window| window.resets_at_datetime.is_some() && window.window_minutes.is_some())?;
    let reset_at = window.resets_at_datetime?;
    let window_minutes = window.window_minutes?;
    if window_minutes <= 0 {
        return None;
    }

    let duration = ChronoDuration::minutes(window_minutes);
    let starts_at = reset_at - duration;
    let total_seconds = duration.num_seconds().max(1);
    let elapsed_seconds = now
        .signed_duration_since(starts_at)
        .num_seconds()
        .clamp(0, total_seconds);
    Some(((elapsed_seconds as f64 / total_seconds as f64) * 100.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::RateLimitWindowDisplay;
    use pretty_assertions::assert_eq;

    #[test]
    fn combines_token_and_reset_percentages() {
        let now = Local::now();
        let token_usage = codex_protocol::protocol::TokenUsage {
            total_tokens: 73_600,
            ..Default::default()
        };
        let token_info = TokenUsageInfo {
            total_token_usage: token_usage.clone(),
            last_token_usage: token_usage,
            model_context_window: Some(100_000),
        };
        let codex_rate_limit = RateLimitSnapshotDisplay {
            limit_name: "codex".to_string(),
            captured_at: now,
            primary: Some(RateLimitWindowDisplay {
                used_percent: 25.0,
                resets_at: None,
                resets_at_datetime: Some(now + ChronoDuration::minutes(150)),
                window_minutes: Some(300),
            }),
            secondary: None,
            credits: None,
        };

        let footer = line(
            Some(&token_info),
            Some(100_000),
            Some(&codex_rate_limit),
            now,
        )
        .expect("session limit footer should render");
        let rendered = footer
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert_eq!(rendered, "70% tokens · 50% reset");
    }

    #[test]
    fn renders_reset_percentage_without_token_usage() {
        let now = Local::now();
        let codex_rate_limit = RateLimitSnapshotDisplay {
            limit_name: "codex".to_string(),
            captured_at: now,
            primary: Some(RateLimitWindowDisplay {
                used_percent: 25.0,
                resets_at: None,
                resets_at_datetime: Some(now + ChronoDuration::minutes(150)),
                window_minutes: Some(300),
            }),
            secondary: None,
            credits: None,
        };

        let footer = line(
            /*token_info*/ None,
            Some(100_000),
            Some(&codex_rate_limit),
            now,
        )
        .expect("session limit footer should render reset progress");
        let rendered = footer
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert_eq!(rendered, "50% reset");
    }

    #[test]
    fn uses_secondary_reset_window_when_primary_has_no_reset_metadata() {
        let now = Local::now();
        let codex_rate_limit = RateLimitSnapshotDisplay {
            limit_name: "codex".to_string(),
            captured_at: now,
            primary: Some(RateLimitWindowDisplay {
                used_percent: 25.0,
                resets_at: None,
                resets_at_datetime: None,
                window_minutes: None,
            }),
            secondary: Some(RateLimitWindowDisplay {
                used_percent: 50.0,
                resets_at: None,
                resets_at_datetime: Some(now + ChronoDuration::minutes(150)),
                window_minutes: Some(300),
            }),
            credits: None,
        };

        let footer = line(
            /*token_info*/ None,
            Some(100_000),
            Some(&codex_rate_limit),
            now,
        )
        .expect("session limit footer should use secondary reset metadata");
        let rendered = footer
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert_eq!(rendered, "50% reset");
    }
}
