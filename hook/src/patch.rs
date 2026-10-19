use crate::symbols;
use anyhow::{Context, Result};

pub fn patch_constant(filename: &str, old: f64, new: f64) -> Result<()> {
    let va = symbols::find_imm64(filename, old)
        .with_context(|| format!("Error finding a constant: {}", old))?;
    (unsafe {
        symbols::write_f64(va, new)
            .with_context(|| format!("Error overwriting a constant: {}", new))
    })?;

    Ok(())
}
