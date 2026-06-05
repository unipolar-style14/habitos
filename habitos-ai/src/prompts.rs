use crate::LlmError;
use std::path::{Path, PathBuf};

/// Loads prompts from disk if present, otherwise falls back to baked-in
/// defaults. Editing a prompt at `<data_dir>/prompts/<name>.md` overrides the
/// default without a recompile.
pub struct PromptLoader {
    prompts_dir: PathBuf,
}

impl PromptLoader {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        Self {
            prompts_dir: data_dir.as_ref().join("prompts"),
        }
    }

    pub fn load(&self, name: &str) -> Result<String, LlmError> {
        let on_disk = self.prompts_dir.join(format!("{name}.md"));
        if let Ok(s) = std::fs::read_to_string(&on_disk) {
            return Ok(s);
        }
        embedded_default(name)
            .map(str::to_string)
            .ok_or_else(|| LlmError::Backend(format!("prompt `{name}` not found")))
    }

    /// Combine the prompt template with a context block.
    pub fn render(&self, name: &str, context: &str) -> Result<String, LlmError> {
        let template = self.load(name)?;
        Ok(format!("{template}\n\n## Context\n\n{context}"))
    }
}

fn embedded_default(name: &str) -> Option<&'static str> {
    Some(match name {
        "plan" => include_str!("../prompts/plan.md"),
        "coach" => include_str!("../prompts/coach.md"),
        "reflect_summary" => include_str!("../prompts/reflect_summary.md"),
        "review_week" => include_str!("../prompts/review_week.md"),
        "ask" => include_str!("../prompts/ask.md"),
        _ => return None,
    })
}
