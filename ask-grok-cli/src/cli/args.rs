use clap::{ArgGroup, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "A stealth CLI to query Grok", long_about = None)]
#[command(group(
    ArgGroup::new("prompt_source")
        .args(["prompt", "prompt_file"])
        .required(true)
))]
pub struct Args {
    /// The prompt you want to send to Grok (inline)
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// Path to a file whose contents are used as the prompt. Sidesteps shell
    /// quoting / argv length limits for prompts longer than ~128 KiB.
    #[arg(long, value_name = "PATH")]
    pub prompt_file: Option<PathBuf>,

    /// Run the browser in headless mode
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub headless: bool,

    /// Skip Drunk-Typist and instant-paste the prompt via CDP `Input.insertText`.
    /// Trades stealth for ~6 min of test-cycle time. Use during selector iteration.
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    pub instant_paste: bool,
}
