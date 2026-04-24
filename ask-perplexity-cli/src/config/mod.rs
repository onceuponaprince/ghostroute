use crate::types::Phase;

pub mod cookies;

// --- parse layer (applied against scraped HTML via scraper crate) ---

pub const ANSWER_CONTAINER: &str = r#"div[id^="markdown-content-"]"#;
pub const SOURCES_CONTAINER: &str = r#"[class*="group/search-side-content"]"#;
pub const SOURCE_ITEM: &str = r#"[class*="group/search-side-content"] a[href]"#;
pub const SOURCE_TITLE: &str = r#"span[class*="font-medium"][class*="text-foreground"]"#;
pub const SOURCE_SNIPPET: &str = r#"span[class*="text-quiet"]"#;
pub const STEP_ITEM: &str = r#"button"#; // filtered post-match on child div text class
pub const STEP_QUERY: &str = r#"div.font-sans.text-quiet.text-sm.select-none.truncate"#;
pub const STEP_PHASE_ICON: &str = r#"svg use"#;

// --- scrape layer (used to drive live page via chromiumoxide selectors) ---

pub const PROMPT_INPUT: &str = r#"div[contenteditable="true"][role="textbox"]"#;
pub const MODEL_BUTTON: &str = r#"button[aria-label="Model"]"#;
pub const TOOLS_BUTTON: &str = r#"button[aria-label="Add files or tools"]"#;
/// The "N sources" button appears after a response — substring match by text handled in Rust.
pub const SOURCES_BUTTON_TEXT: &str = "sources";
pub const LOGIN_WALL_HREF_SUBSTRING: &str = "/sign-in";

// --- focus routing ---

pub fn focus_url(focus: &str) -> &'static str {
    match focus {
        "academic" => "https://www.perplexity.ai/academic",
        "finance" => "https://www.perplexity.ai/finance",
        "health" => "https://www.perplexity.ai/health",
        "patents" => "https://www.perplexity.ai/patents",
        _ => "https://www.perplexity.ai/",
    }
}

// --- model → Radix menuitemradio label (prefix match tolerated via contains()) ---

pub fn model_label(model: &str) -> Option<&'static str> {
    match model {
        "best" => Some("Best"),
        "sonar" => Some("Sonar"),
        "gpt" => Some("GPT"),
        "gemini" => Some("Gemini"),
        "claude" => Some("Claude"),
        "kimi" => Some("Kimi"),
        "nemotron" => Some("Nemotron"),
        _ => None,
    }
}

// --- tool → Radix menuitemradio label ---

pub fn tool_label(tool: &str) -> Option<&'static str> {
    match tool {
        "deep-research" => Some("Deep research"),
        _ => None,
    }
}

// --- phase-icon href → Phase enum ---

pub fn phase_by_icon(icon_ref: &str) -> Phase {
    match icon_ref {
        "#pplx-icon-blocks" => Phase::Identifying,
        "#pplx-icon-world-search" => Phase::Searching,
        "#pplx-icon-bolt" => Phase::Insights,
        _ => Phase::Other,
    }
}
