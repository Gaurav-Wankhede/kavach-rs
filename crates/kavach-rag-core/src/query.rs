/// Input to the matcher. Owned strings so gates can build it from hook input
/// without lifetime plumbing.
#[derive(Debug, Clone)]
pub struct Query {
    file_path: String,
    tokens: Vec<String>,
    intent: String,
}

impl Query {
    pub fn new(file_path: impl Into<String>, raw_text: &str, intent: impl Into<String>) -> Self {
        Self {
            file_path: file_path.into(),
            tokens: tokenize(raw_text),
            intent: intent.into(),
        }
    }

    #[must_use]
    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    #[must_use]
    pub fn tokens(&self) -> &[String] {
        &self.tokens
    }

    #[must_use]
    pub fn intent(&self) -> &str {
        &self.intent
    }
}

fn tokenize(raw: &str) -> Vec<String> {
    raw.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
        .collect()
}
