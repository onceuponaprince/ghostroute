pub const INPUT_TIMEOUT_MS: f64 = 12_000.0;
pub const RESPONSE_TIMEOUT_MS: f64 = 900_000.0;

// `.tiptap.ProseMirror` are the editor library's own root classes — stable
// across xAI's UI shuffles unless they swap editors. Far more durable than
// matching only on `contenteditable=true` + `tabindex=0`, which the page
// has multiple of (search box, hidden drafts, the actual input).
pub const INPUT_SELECTOR: &str = "div.tiptap.ProseMirror[contenteditable=\"true\"]";
// `id^="response-"` is the parent container per turn — both user and
// assistant get one. With the 2-element gate in `wait_for_stable_response_text`,
// matching the parent (rather than its inner `.message-bubble`) lets `innerText`
// capture EVERYTHING in the assistant turn: prose paragraphs AND list items
// AND headings AND code blocks. The previous narrower selector only caught
// the prose-paragraph subdiv, dropping bullets that render as siblings.
pub const RESPONSE_SELECTOR: &str = "div[id^=\"response-\"]";
