use super::ContextualUserFragment;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskMemory {
    body: String,
}

impl TaskMemory {
    pub(crate) fn new(body: impl Into<String>) -> Self {
        Self { body: body.into() }
    }
}

impl ContextualUserFragment for TaskMemory {
    const ROLE: &'static str = "user";
    const START_MARKER: &'static str = "<task_memory>";
    const END_MARKER: &'static str = "</task_memory>";

    fn body(&self) -> String {
        format!("\n{}\n", self.body.trim())
    }
}
