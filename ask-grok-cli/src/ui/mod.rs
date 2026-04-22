use colored::Colorize;

pub fn print_banner() {
    eprintln!(
        "{}",
        r#"
      _____           _      ___  ___  _ 
     |  __ \         | |     |  \/  | | |
     | |  \/_ __ ___ | | __  | .  . | | |
     | | __| '__/ _ \| |/ /  | |\/| | | |
     | |_\ \ | | (_) |   <   | |  | | |_|
      \____/_|  \___/|_|\_\  \_|  |_/ (_)
    "#
        .green()
        .bold()
    );

    eprintln!("{} Initiating stealth sequence...", "[System]".blue().bold());
    eprintln!("{} Booting Chromium engine...", "[Status]".yellow());
}
