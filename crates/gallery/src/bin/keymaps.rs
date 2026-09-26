//! Writes every component's default keyboard shortcuts as JSON for the docs site.

use std::io::Write;

fn main() -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(&gallery::keymaps::all())?;
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{json}")
}
