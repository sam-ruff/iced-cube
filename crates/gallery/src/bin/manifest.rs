//! Writes every story's metadata and source as JSON for the docs site.

use std::io::Write;

fn main() -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(gallery::stories::ALL)?;
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{json}")
}
