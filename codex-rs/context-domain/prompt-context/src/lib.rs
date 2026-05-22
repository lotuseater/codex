//! Prompt context abstractions shared by context collection and assembly code.

#![deny(private_bounds, private_interfaces, unreachable_pub)]

use std::future::Future;
use std::path::Path;

use codex_utils_absolute_path::AbsolutePathBuf;

/// Default number of paths a context-pack renderer should include.
pub const DEFAULT_CONTEXT_PACK_PATH_BUDGET: usize = 16;

/// One protocol-neutral item that can be included in prompt context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptContextItem {
    /// Stable source label, such as a file path, memory key, or tool name.
    pub source: String,
    /// Textual content contributed by the source.
    pub body: String,
}

/// Supplies prompt context items to a prompt assembly layer.
///
/// Implementations should collect already-authorized context and leave token
/// selection, ordering, and model serialization to callers.
pub trait PromptContextProvider {
    /// Returns context items currently available for prompt assembly.
    fn prompt_context(&self) -> Vec<PromptContextItem>;
}

/// Request to render a repository context pack for a prompt.
///
/// Implementations should treat this as a domain request and perform any file
/// walking, ranking, or formatting in a concrete renderer outside the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextPackRequest<'a> {
    /// Repository root used to discover candidate context paths.
    pub project_root: &'a Path,
    /// User prompt that drives path selection and rendering.
    pub prompt: &'a str,
    /// Maximum number of paths the renderer should include.
    pub path_budget: usize,
}

impl<'a> ContextPackRequest<'a> {
    /// Builds a request with the default path budget.
    pub fn new(project_root: &'a Path, prompt: &'a str) -> Self {
        Self {
            project_root,
            prompt,
            path_budget: DEFAULT_CONTEXT_PACK_PATH_BUDGET,
        }
    }
}

/// Renders repository context packs for prompt injection.
///
/// Implementations own concrete repository inspection and formatting details.
/// Callers should depend on this port when they only need optional context text.
pub trait ContextPackRenderer {
    /// Renders context text for the request, or returns `None` when no pack applies.
    fn render_context_pack(&self, request: &ContextPackRequest<'_>) -> Option<String>;
}

/// Detects context-pack markup in already assembled context text.
///
/// Implementations should keep marker parsing compatible with their renderer so
/// callers can remove stale injected context without depending on renderer code.
pub trait ContextPackMarker {
    /// Returns whether the supplied message contains context-pack markup.
    fn has_context_pack(&self, message: &str) -> bool;
}

/// Classifies prompts that explicitly request repository routing or exploration.
///
/// Implementations should keep classifier heuristics near their concrete
/// context-pack implementation while exposing only this prompt-level decision.
pub trait RepoRoutingClassifier {
    /// Returns whether the prompt explicitly asks for repository routing context.
    fn is_explicit_repo_routing_prompt(&self, prompt: &str) -> bool;
}

/// Request to fetch project-problem memory context for prompt injection.
///
/// Implementations should perform authorization, lookup, ranking, and rendering
/// behind the port; callers only provide the query shape and limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectProblemContextRequest<'a> {
    /// Codex home used by the concrete memory provider.
    pub codex_home: &'a AbsolutePathBuf,
    /// Repository root used to scope project memory lookup.
    pub project_root: &'a Path,
    /// User prompt used as the memory query.
    pub prompt: &'a str,
    /// Maximum number of matching memories to render.
    pub max_matches: usize,
}

/// Supplies rendered project-problem memory context for prompt injection.
///
/// Implementations own storage access and rendering. They should return a
/// `Send` future so callers can use the port from asynchronous session code
/// without depending on the concrete memory runtime.
pub trait ProjectProblemContextProvider {
    /// Returns rendered project-problem context, or `None` when no context applies.
    fn project_problem_context(
        &self,
        request: ProjectProblemContextRequest<'_>,
    ) -> impl Future<Output = Option<String>> + Send;
}
