use clap::{ArgAction, Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Native Rust CLI for Perplexity.ai via chromiumoxide",
    long_about = None,
)]
pub struct Args {
    /// The prompt to send to Perplexity.
    pub prompt: String,

    /// Model to use.
    #[arg(long, value_enum, default_value_t = Model::Best)]
    pub model: Model,

    /// Topic focus / entry URL.
    #[arg(long, value_enum, default_value_t = Focus::Web)]
    pub focus: Focus,

    /// Continue an existing thread by its UUID-like slug.
    #[arg(long)]
    pub thread: Option<String>,

    /// Enable Deep Research (synchronous — blocks up to 30 min).
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub deep: bool,

    /// Include raw HTML blobs in the output JSON.
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub raw: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
#[value(rename_all = "kebab-case")]
pub enum Model {
    Best,
    Sonar,
    Gpt,
    Gemini,
    Claude,
    Kimi,
    Nemotron,
}

impl Model {
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::Best => "best",
            Model::Sonar => "sonar",
            Model::Gpt => "gpt",
            Model::Gemini => "gemini",
            Model::Claude => "claude",
            Model::Kimi => "kimi",
            Model::Nemotron => "nemotron",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
#[value(rename_all = "kebab-case")]
pub enum Focus {
    Web,
    Academic,
    Finance,
    Health,
    Patents,
}

impl Focus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Focus::Web => "web",
            Focus::Academic => "academic",
            Focus::Finance => "finance",
            Focus::Health => "health",
            Focus::Patents => "patents",
        }
    }
}
