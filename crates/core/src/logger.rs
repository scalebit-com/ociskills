use chrono::Local;
use colored::*;
use traits::Logger;

pub struct ConsoleLogger {
    use_color: bool,
    verbose: bool,
    quiet: bool,
}

impl ConsoleLogger {
    pub fn new(use_color: bool, verbose: bool, quiet: bool) -> Self {
        Self {
            use_color,
            verbose,
            quiet,
        }
    }

    fn format_message(&self, level: &str, message: &str) -> String {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let formatted = format!("{} {} {}", timestamp, level, message);

        if !self.use_color {
            return formatted;
        }

        match level {
            "INFO" => formatted.green().to_string(),
            "WARN" => formatted.yellow().to_string(),
            "ERROR" => formatted.red().to_string(),
            "DEBUG" => formatted.blue().to_string(),
            _ => formatted,
        }
    }
}

impl Logger for ConsoleLogger {
    fn debug(&self, message: &str) {
        if self.verbose {
            eprintln!("{}", self.format_message("DEBUG", message));
        }
    }

    fn info(&self, message: &str) {
        if !self.quiet {
            println!("{}", self.format_message("INFO ", message));
        }
    }

    fn warn(&self, message: &str) {
        if !self.quiet {
            eprintln!("{}", self.format_message("WARN ", message));
        }
    }

    fn error(&self, message: &str) {
        eprintln!("{}", self.format_message("ERROR", message));
    }
}
