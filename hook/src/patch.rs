use anyhow::{Context, Result, bail};
use windows::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows::Win32::System::Memory::{
    PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect,
};
use windows::Win32::System::Threading::GetCurrentProcess;

use crate::pe_structs;
use crate::symbols;

pub fn find_imm64(func_name: &str, old: f64) -> Result<pe_structs::Va> {
    let func = symbols::get_function_address(func_name)
        .with_context(|| format!("Cant find function {}", func_name))?;
    let code =
        unsafe { std::slice::from_raw_parts(func.start.as_ptr::<u8>(), func.end - func.start) };

    // TODO: this only find imm64 moved in RAX, includes other registers
    // TODO: ensure rdata constants are NOT changed, since they are shared
    const INSTRUCTION_LEN: usize = 10;
    let mut pat: [u8; INSTRUCTION_LEN] =
        [0x48u8, 0xB8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    pat[2..].copy_from_slice(&old.to_bits().to_le_bytes());

    let hits: Vec<usize> = code
        .windows(INSTRUCTION_LEN)
        .enumerate()
        .filter(|(_, w)| *w == pat.as_slice())
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [i] => Ok(func.start + *i + 2),
        h => bail!("expected 1 match for {old}, found {}", h.len()),
    }
}

/// # Safety
///
/// addr has to be the location of a constant found thanks to find_imm64, addr points to 8 bytes of code that can safely be overwritten
pub unsafe fn write_f64(addr: pe_structs::Va, v: f64) -> Result<()> {
    let mut prev = PAGE_PROTECTION_FLAGS(0);
    unsafe {
        VirtualProtect(addr.as_ptr(), 8, PAGE_EXECUTE_READWRITE, &mut prev)?;
        std::ptr::write_unaligned(addr.as_mut_ptr(), v.to_bits());
        VirtualProtect(addr.as_ptr(), 8, prev, &mut prev)?;
        FlushInstructionCache(GetCurrentProcess(), Some(addr.as_ptr()), 8)?;
    }
    Ok(())
}

pub fn patch_constant(filename: &str, old: f64, new: f64) -> Result<()> {
    let va =
        find_imm64(filename, old).with_context(|| format!("Error finding a constant: {}", old))?;
    (unsafe {
        write_f64(va, new).with_context(|| format!("Error overwriting a constant: {}", new))
    })?;

    Ok(())
}
