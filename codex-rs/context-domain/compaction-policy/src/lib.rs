//! Policy abstractions for deciding when prompt context should be compacted.
//!
//! The crate is intentionally independent from compaction executors and model
//! clients; it only represents policy inputs and decisions.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

/// Default fraction of the usable context window that triggers context reduction.
pub const DEFAULT_TRIGGER_CONTEXT_PERCENT: u8 = 20;

/// Default number of completed regular turns to wait after a reduction.
pub const DEFAULT_TURN_COOLDOWN: u32 = 24;

/// Decision returned by a compaction policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompactionDecision {
    /// Keep the context as-is.
    Keep,
    /// Compact the context toward the requested token target.
    Compact { target_tokens: u64 },
}

/// Decides whether a context value should be compacted.
///
/// Implementations should be deterministic for the same policy configuration
/// and context input. Executing the compaction is owned by a separate layer.
pub trait CompactionPolicy {
    /// Context representation evaluated by the policy.
    type Context;

    /// Evaluates the supplied context and returns a compaction decision.
    fn evaluate(&self, context: &Self::Context) -> CompactionDecision;
}

/// Threshold and cooldown settings for context reduction decisions.
///
/// Implementations that execute compaction should translate this domain policy
/// into their concrete runtime representation instead of exposing executor
/// types to callers that only need to assemble context inputs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextReductionPolicy {
    trigger_context_percent: u8,
    turn_cooldown: u32,
}

impl ContextReductionPolicy {
    /// Builds a context reduction policy from explicit threshold and cooldown values.
    pub fn new(trigger_context_percent: u8, turn_cooldown: u32) -> Self {
        Self {
            trigger_context_percent,
            turn_cooldown,
        }
    }

    /// Returns the visible context percentage that should trigger reduction.
    pub fn trigger_context_percent(self) -> u8 {
        self.trigger_context_percent
    }

    /// Returns the number of completed regular turns to wait after reduction.
    pub fn turn_cooldown(self) -> u32 {
        self.turn_cooldown
    }
}

impl Default for ContextReductionPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_TRIGGER_CONTEXT_PERCENT, DEFAULT_TURN_COOLDOWN)
    }
}

/// Domain reason for reducing context.
///
/// Concrete reducers should preserve this reason when mapping to analytics,
/// post-sampling actions, or model-facing compaction prompts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextReductionReason {
    /// The request reached the configured hard context limit.
    ContextLimit,
    /// The semantic policy selected a checkpoint boundary.
    SemanticCheckpoint,
    /// Context pressure was observed before the hard limit was reached.
    EarlyContextPressure,
}

/// User-selected budget mode for automatic compaction.
///
/// Runtime adapters should convert this into the executor-specific budget mode
/// at the boundary where concrete compaction is invoked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutoCompactBudgetMode {
    /// Use the standard model-derived compaction limit.
    Standard,
    /// Use the slower, smaller runtime-window-aware compaction limit.
    Slow,
}

/// Model-provided limits needed to derive automatic compaction thresholds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelAutoCompactLimits {
    /// Optional model-specified automatic compaction token limit.
    pub auto_compact_token_limit: Option<i64>,
    /// Optional model context window.
    pub context_window: Option<i64>,
}

/// Domain input for semantic compaction decisions.
///
/// Context collection code should build this protocol-neutral input and let a
/// concrete reducer adapter translate it into the executor's native type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticCompactInput {
    /// Whether semantic auto-compaction is enabled for the turn.
    pub enabled: bool,
    /// Threshold and cooldown policy for the current request.
    pub policy: ContextReductionPolicy,
    /// Total input usage observed for the current conversation.
    pub total_usage_tokens: i64,
    /// Active automatic compaction token limit.
    pub auto_compact_limit: i64,
    /// Optional visible-context usage percentage reported by the runtime.
    pub visible_context_percent_used: Option<i64>,
}

/// Per-turn usage delta used to update semantic compaction state.
///
/// Concrete state machines should treat this as an immutable event describing
/// the completed turn and own any persistence or cooldown side effects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticCompactTurnInput {
    /// Input tokens that were not served from cache.
    pub non_cached_input_tokens: i64,
    /// Output tokens produced during the turn.
    pub output_tokens: i64,
    /// Number of tool calls observed during the turn.
    pub tool_calls: u64,
    /// Whether a Git commit was observed during the turn.
    pub git_commit_observed: bool,
    /// Whether this was a continuation turn.
    pub is_continuation_turn: bool,
}
