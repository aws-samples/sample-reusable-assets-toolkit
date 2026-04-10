use std::path::Path;

use anyhow::Result;
use rat_cli::chunk;

pub fn handle(file: &str) -> Result<()> {
    let path = Path::new(file).canonicalize()?;
    let chunks = chunk::chunk_file(&path)?;
    for (i, c) in chunks.iter().enumerate() {
        println!(
            "--- chunk {} (L{}-L{}) {} ---",
            i + 1,
            c.start_line,
            c.end_line,
            c.symbol_name.as_deref().unwrap_or("")
        );
        if !c.imports.is_empty() {
            println!("[imports]\n{}\n", c.imports);
        }
        println!("{}", c.content);
        println!();
    }
    println!("{} chunks total", chunks.len());
    Ok(())
}
