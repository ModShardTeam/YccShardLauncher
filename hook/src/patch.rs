use crate::pe::addr;
use crate::runtime::symbols;
use anyhow::{Context, Result, bail};
use windows::Win32::System::{
    Diagnostics::Debug::FlushInstructionCache,
    Memory::{PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect},
    Threading::GetCurrentProcess,
};

pub fn patch_constant(func_name: &str, old: f64, new: f64) -> Result<()> {
    let va = find_imm64(func_name, old)
        .with_context(|| format!("error in {} finding {}", func_name, old))?;
    (unsafe {
        write_f64(va, new).with_context(|| format!("error in {} overwriting {}", func_name, new))
    })?;

    Ok(())
}

pub fn find_imm64(func_name: &str, old: f64) -> Result<addr::Va> {
    let func = symbols::get_function_address(func_name)
        .with_context(|| format!("cannot find function {}", func_name))?;
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
        h => bail!(
            "in {} expected 1 match for {}, found {}",
            func_name,
            old,
            h.len()
        ),
    }
}

/// # Safety
///
/// addr has to be the location of a constant found thanks to find_imm64, addr points to 8 bytes of code that can safely be overwritten
pub unsafe fn write_f64(addr: addr::Va, v: f64) -> Result<()> {
    let mut prev = PAGE_PROTECTION_FLAGS(0);
    unsafe {
        VirtualProtect(addr.as_ptr(), 8, PAGE_EXECUTE_READWRITE, &mut prev)?;
        std::ptr::write_unaligned(addr.as_mut_ptr(), v.to_bits());
        VirtualProtect(addr.as_ptr(), 8, prev, &mut prev)?;
        FlushInstructionCache(GetCurrentProcess(), Some(addr.as_ptr()), 8)?;
    }
    Ok(())
}
