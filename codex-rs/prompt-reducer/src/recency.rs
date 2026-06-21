//! Recency-tier model for graduated prompt reduction.
//!
//! The reducer maps every text slot of the prompt clone to exactly one
//! [`RecencyTier`] by its distance from the END of the conversation (most
//! recent first). Each tier decides how aggressively that slot may be reduced:
//! whether it is fully preserved, only eligible for the `recent_*` digest
//! categories, or eligible for every category, plus per-tier multipliers that
//! scale the size thresholds and kept-excerpt length.
//!
//! Two baked-in tier lists are provided:
//! - [`conservative_tiers`] reproduces today's binary behaviour
//!   (`preserve_recent_items` recent-only slots, then all-categories for the
//!   rest). It is byte-identical to the pre-tier reducer.
//! - [`recency_weighted_tiers`] is the improved default: a fully-preserved
//!   newest band, a recent-only window, a standard mid band, then an
//!   aggressively-reduced tail.

use std::collections::BTreeSet;

/// How a recency tier gates reduction for the slots it covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierKind {
    /// Skip ALL reduction for slots in this tier (keep them verbatim).
    Preserve,
    /// Only the narrower `recent_*` digest categories are eligible
    /// (matches the historical "recent prompt item" behaviour).
    RecentOnly,
    /// Every reduction category is eligible (historical "old prompt item").
    All,
}

/// One recency tier, most-recent-first within a [`RecencyPolicy`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecencyTier {
    /// Number of text slots this tier covers. The LAST tier in a policy uses
    /// [`usize::MAX`] to mean "all remaining slots".
    pub slot_count: usize,
    /// Reduction gating for this tier.
    pub kind: TierKind,
    /// Multiplies the per-category `min_chars` + `min_saved_tokens` thresholds
    /// (`> 1.0` = gentler / reduce less, `< 1.0` = harsher / reduce more).
    pub threshold_mult: f32,
    /// Multiplies the kept-excerpt length for digests in this tier
    /// (`< 1.0` = shorter excerpts for older items).
    pub excerpt_mult: f32,
}

impl RecencyTier {
    /// True when this tier preserves its slots from all reduction.
    pub fn is_preserve(&self) -> bool {
        matches!(self.kind, TierKind::Preserve)
    }

    /// True when this tier restricts slots to the `recent_*` categories.
    pub fn is_recent_only(&self) -> bool {
        matches!(self.kind, TierKind::RecentOnly)
    }

    /// True when every reduction category is eligible for this tier. This is the
    /// historical "non-recent / old prompt item" condition that gates the
    /// stale-output bundles.
    pub fn is_all(&self) -> bool {
        matches!(self.kind, TierKind::All)
    }
}

/// An ordered (most-recent-first) list of recency tiers plus a resolver that
/// maps a slot index to its tier given the total number of text slots.
#[derive(Debug, Clone, PartialEq)]
pub struct RecencyPolicy {
    tiers: Vec<RecencyTier>,
}

impl RecencyPolicy {
    /// Build a policy from a most-recent-first tier list. An empty list falls
    /// back to a single all-categories tier so the reducer always resolves.
    pub fn new(tiers: Vec<RecencyTier>) -> Self {
        if tiers.is_empty() {
            return Self {
                tiers: vec![RecencyTier {
                    slot_count: usize::MAX,
                    kind: TierKind::All,
                    threshold_mult: 1.0,
                    excerpt_mult: 1.0,
                }],
            };
        }
        Self { tiers }
    }

    /// Resolve the tier for a slot.
    ///
    /// `slot_index` is the 0-based index of the text slot in document order;
    /// `total_text_slots` is the count of all text slots. Distance from the
    /// end (`total - 1 - slot_index`) selects the tier: the first tier covers
    /// the newest `tiers[0].slot_count` slots, the next the following band, and
    /// so on; the final tier covers everything older.
    pub fn tier_for(&self, slot_index: usize, total_text_slots: usize) -> RecencyTier {
        let distance_from_end = total_text_slots
            .saturating_sub(1)
            .saturating_sub(slot_index);
        let mut covered = 0usize;
        for tier in &self.tiers {
            covered = covered.saturating_add(tier.slot_count);
            if distance_from_end < covered {
                return *tier;
            }
        }
        // Defensive: a well-formed policy ends in a usize::MAX tier, so this is
        // unreachable, but fall back to the last (oldest) tier.
        self.tiers.last().copied().unwrap_or(RecencyTier {
            slot_count: usize::MAX,
            kind: TierKind::All,
            threshold_mult: 1.0,
            excerpt_mult: 1.0,
        })
    }

    /// Boundary slot index separating "recent" (non-`All`) newest slots from
    /// the older `All`-category slots, given the total text-slot count.
    ///
    /// Slots `>= boundary` are gated to a narrower-than-`All` tier (the leading
    /// `Preserve` / `RecentOnly` tiers, which are always newest-first); slots
    /// `< boundary` are eligible for every category. This reproduces the
    /// historical `recent_text_start` value (`total - preserve_recent_items`)
    /// for the conservative policy and is consumed by the stale-output bundles.
    pub fn all_categories_boundary(&self, total_text_slots: usize) -> usize {
        let mut leading_recent = 0usize;
        for tier in &self.tiers {
            if tier.is_all() {
                break;
            }
            leading_recent = leading_recent.saturating_add(tier.slot_count);
        }
        total_text_slots.saturating_sub(leading_recent)
    }
}

/// Conservative tier list: `preserve_recent_items` recent-only slots, then
/// all-categories for the remainder. Reproduces the historical binary boundary
/// byte-for-byte (all multipliers are `1.0`).
pub fn conservative_tiers(preserve_recent_items: usize) -> Vec<RecencyTier> {
    vec![
        RecencyTier {
            slot_count: preserve_recent_items,
            kind: TierKind::RecentOnly,
            threshold_mult: 1.0,
            excerpt_mult: 1.0,
        },
        RecencyTier {
            slot_count: usize::MAX,
            kind: TierKind::All,
            threshold_mult: 1.0,
            excerpt_mult: 1.0,
        },
    ]
}

/// Default sizes for the recency-weighted tier list.
pub const DEFAULT_PRESERVE_RECENT_TIER: usize = 3;
pub const DEFAULT_RECENT_WINDOW_TIER: usize = 6;
pub const DEFAULT_MID_WINDOW_TIER: usize = 12;
pub const DEFAULT_OLD_THRESHOLD_MULT: f32 = 0.5;
pub const DEFAULT_OLD_EXCERPT_MULT: f32 = 0.6;

/// Recency-weighted (graduated) tier list — the improved default.
///
/// `[Preserve{preserve}, RecentOnly{recent}, All{mid, 1.0, 1.0}, All{REST, old_thr, old_exc}]`:
/// the newest `preserve` slots are kept verbatim (more recent detail than the
/// old behaviour), a `recent` recent-only window follows, then a `mid`
/// standard-reduction band, then everything older is reduced aggressively
/// (lower thresholds + shorter excerpts = "cut older more").
pub fn recency_weighted_tiers(
    preserve: usize,
    recent: usize,
    mid: usize,
    old_threshold_mult: f32,
    old_excerpt_mult: f32,
) -> Vec<RecencyTier> {
    vec![
        RecencyTier {
            slot_count: preserve,
            kind: TierKind::Preserve,
            threshold_mult: 1.0,
            excerpt_mult: 1.0,
        },
        RecencyTier {
            slot_count: recent,
            kind: TierKind::RecentOnly,
            threshold_mult: 1.0,
            excerpt_mult: 1.0,
        },
        RecencyTier {
            slot_count: mid,
            kind: TierKind::All,
            threshold_mult: 1.0,
            excerpt_mult: 1.0,
        },
        RecencyTier {
            slot_count: usize::MAX,
            kind: TierKind::All,
            threshold_mult: old_threshold_mult,
            excerpt_mult: old_excerpt_mult,
        },
    ]
}

/// Plain-data, reducer-owned options for building a recency-weighted config.
///
/// This is intentionally NOT the config crate's type: the core call sites
/// translate `codex-config` values into this struct so the reducer stays free
/// of any `codex-config` / `codex-core` dependency.
#[derive(Debug, Clone)]
pub struct RecencyWeightedOpts {
    /// Size of the fully-preserved newest tier.
    pub preserve_recent_items: usize,
    /// Size of the recent-only window tier.
    pub recent_window_items: usize,
    /// Size of the standard-reduction mid tier.
    pub mid_window_items: usize,
    /// Threshold multiplier for the oldest (tail) tier.
    pub old_threshold_mult: f32,
    /// Excerpt-length multiplier for the oldest (tail) tier.
    pub old_excerpt_mult: f32,
    /// Canonical category names that must never be reduced (the "list of cases").
    pub disabled_categories: Vec<String>,
    /// Optional global override for the base `min_reduce_chars` threshold.
    pub min_reduce_chars: Option<usize>,
    /// Optional global override for the base `min_saved_tokens` threshold.
    pub min_saved_tokens: Option<usize>,
}

impl Default for RecencyWeightedOpts {
    fn default() -> Self {
        Self {
            preserve_recent_items: DEFAULT_PRESERVE_RECENT_TIER,
            recent_window_items: DEFAULT_RECENT_WINDOW_TIER,
            mid_window_items: DEFAULT_MID_WINDOW_TIER,
            old_threshold_mult: DEFAULT_OLD_THRESHOLD_MULT,
            old_excerpt_mult: DEFAULT_OLD_EXCERPT_MULT,
            disabled_categories: Vec::new(),
            min_reduce_chars: None,
            min_saved_tokens: None,
        }
    }
}

/// Parse a list of category-name strings into a deduplicated set. Unknown names
/// are kept as-is (forward-compatible); matching is exact against canonical
/// reduction category names (see `CANONICAL_CATEGORY_NAMES`).
pub fn parse_disabled_categories<I, S>(names: I) -> BTreeSet<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    names
        .into_iter()
        .map(|name| name.as_ref().trim().to_string())
        .filter(|name| !name.is_empty())
        .collect()
}

/// All canonical reduction category names. Each is the stable snake_case
/// identifier used both as the on-prompt `[prompt reduction: <name>]` reason
/// and as the value accepted in `disabled_categories`.
pub const CANONICAL_CATEGORY_NAMES: &[&str] = &[
    "duplicate_block",
    "workflow_batch_success_digest",
    "build_status_digest",
    "search_result_digest",
    "source_read_digest",
    "diff_hunk_digest",
    "compiler_diagnostic_digest",
    "self_review_inventory",
    "plan_review_prompt",
    "completed_plan_checkpoint",
    "single_use_helper_prompt",
    "proposed_plan_digest",
    "review_result_digest",
    "assistant_findings_digest",
    "context_pack_digest",
    "path_inventory",
    "assistant_status_digest",
    "json_digest",
    "command_log_digest",
    "recoverable_prior_context",
    "recent_assistant_status_digest",
    "recent_build_status_digest",
    "recent_search_result_digest",
    "recent_path_inventory",
    "recent_json_digest",
    "recent_command_log_digest",
    "single_use_self_review_prompt",
    "single_use_plan_review_prompt",
    "single_use_completed_plan_checkpoint",
    "single_use_proposed_plan",
    "single_use_prompt_reduction_notice",
    "subagent_notification_digest",
    "single_use_subagent_status_notice",
    "tool_search_digest",
    "short_tool_output_bundle",
    "short_assistant_status_bundle",
    "stale_reduction_notice_bundle",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn tier(slot_count: usize, kind: TierKind) -> RecencyTier {
        RecencyTier {
            slot_count,
            kind,
            threshold_mult: 1.0,
            excerpt_mult: 1.0,
        }
    }

    #[test]
    fn conservative_policy_matches_binary_boundary() {
        // preserve_recent_items = 4, total = 10 -> slots 6,7,8,9 are recent-only.
        let policy = RecencyPolicy::new(conservative_tiers(4));
        let total = 10;
        for slot in 0..6 {
            assert_eq!(
                policy.tier_for(slot, total).kind,
                TierKind::All,
                "slot {slot}"
            );
        }
        for slot in 6..10 {
            assert_eq!(
                policy.tier_for(slot, total).kind,
                TierKind::RecentOnly,
                "slot {slot}"
            );
        }
    }

    #[test]
    fn conservative_with_zero_preserve_is_all() {
        let policy = RecencyPolicy::new(conservative_tiers(0));
        for slot in 0..5 {
            assert_eq!(policy.tier_for(slot, 5).kind, TierKind::All);
        }
    }

    #[test]
    fn recency_weighted_bands_resolve_by_distance() {
        // preserve 3, recent 6, mid 12, rest. total large so all bands present.
        let policy = RecencyPolicy::new(recency_weighted_tiers(3, 6, 12, 0.5, 0.6));
        let total = 40;
        // newest 3 -> Preserve (slots 37,38,39)
        for slot in 37..40 {
            assert!(policy.tier_for(slot, total).is_preserve(), "slot {slot}");
        }
        // next 6 -> RecentOnly (slots 31..37)
        for slot in 31..37 {
            assert!(policy.tier_for(slot, total).is_recent_only(), "slot {slot}");
        }
        // next 12 -> All, mult 1.0 (slots 19..31)
        let mid = policy.tier_for(25, total);
        assert_eq!(mid.kind, TierKind::All);
        assert_eq!(mid.threshold_mult, 1.0);
        // oldest -> All, aggressive mults (slot 0)
        let old = policy.tier_for(0, total);
        assert_eq!(old.kind, TierKind::All);
        assert_eq!(old.threshold_mult, 0.5);
        assert_eq!(old.excerpt_mult, 0.6);
    }

    #[test]
    fn empty_policy_falls_back_to_all() {
        let policy = RecencyPolicy::new(vec![]);
        assert_eq!(policy.tier_for(0, 3).kind, TierKind::All);
    }

    #[test]
    fn short_history_keeps_everything_in_first_tier() {
        // total smaller than preserve tier: every slot is in the newest tier.
        let policy = RecencyPolicy::new(recency_weighted_tiers(3, 6, 12, 0.5, 0.6));
        for slot in 0..2 {
            assert!(policy.tier_for(slot, 2).is_preserve(), "slot {slot}");
        }
    }

    #[test]
    fn parse_disabled_dedups_and_trims() {
        let set =
            parse_disabled_categories(["source_read_digest", " json_digest ", "", "json_digest"]);
        assert_eq!(set.len(), 2);
        assert!(set.contains("source_read_digest"));
        assert!(set.contains("json_digest"));
    }

    #[test]
    fn single_tier_helper_compiles() {
        let _ = tier(1, TierKind::All);
    }
}
