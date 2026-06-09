//! `spec-spine init` — scaffold a new adopter (spec 006).
//!
//! Core returns the files as data ([`spec_spine_core::scaffold_init`]); this is
//! where they are written. Without `--force`, a pre-existing file is skipped (not
//! an error — `init` is idempotent); with `--force`, every file is overwritten.

use std::fs;
use std::path::Path;

use spec_spine_core::scaffold_init;
use spec_spine_types::Error;

use crate::load_repo_config;

pub fn run(repo: &Path, force: bool) -> Result<u8, Error> {
    let cfg = load_repo_config(repo)?;
    let scaffold = scaffold_init(&cfg)?;

    let mut written = 0usize;
    let mut skipped = 0usize;

    for file in &scaffold.files {
        let abs = repo.join(&file.rel_path);
        if abs.exists() && !force && !file.overwrite {
            println!("  skip (exists): {}", file.rel_path);
            skipped += 1;
            continue;
        }
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::Io(format!("create {}: {e}", parent.display())))?;
        }
        fs::write(&abs, &file.contents)
            .map_err(|e| Error::Io(format!("write {}: {e}", abs.display())))?;
        println!("  write: {}", file.rel_path);
        written += 1;
    }

    println!(
        "spec-spine init: {written} file(s) written, {skipped} skipped{}. Next: customize \
         specs/000-bootstrap/spec.md, then run `spec-spine compile`.",
        if force { " (--force)" } else { "" }
    );
    Ok(0)
}
