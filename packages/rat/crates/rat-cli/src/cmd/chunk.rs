use std::path::Path;

use anyhow::Result;
use crate::{chunk, highlight};

pub fn handle(file: &str) -> Result<()> {
    let path = Path::new(file).canonicalize()?;
    let chunks = chunk::chunk_file(&path)?;
    let lang = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(highlight::language_from_ext);

    for (i, c) in chunks.iter().enumerate() {
        println!(
            "--- chunk {} (L{}-L{}) {} ---",
            i + 1,
            c.start_line,
            c.end_line,
            c.symbol_name.as_deref().unwrap_or("")
        );
        if !c.imports.is_empty() {
            println!("{}", highlight::highlight(&c.imports, lang));
        }
        println!("{}", highlight::highlight(&c.content, lang));
        println!();
    }
    println!("{} chunks total", chunks.len());
    Ok(())
}
