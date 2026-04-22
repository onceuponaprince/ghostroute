use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "A stealth CLI to query Grok", long_about = None)]
pub struct Args {
    /// The prompt you want to send to Grok
    #[arg(short, long)]
    pub prompt: String,
    /// Run the browser in headless mode
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub headless: bool,
}
