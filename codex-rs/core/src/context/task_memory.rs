use super::ContextualUserFragment;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TaskMemory {
    body: String,
}

impl ContextualUserFragment for TaskMemory {
    fn role() -> &'static str {
        "user"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("<task_memory>", "</task_memory>")
    }

    fn body(&self) -> String {
        format!("\n{}\n", self.body.trim())
    }
}
