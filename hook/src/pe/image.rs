use std::{range::Range, slice};

use anyhow::{Context, Result, bail};
use windows::Win32::System::{
    Diagnostics::Debug::{
        IMAGE_DIRECTORY_ENTRY_EXCEPTION, IMAGE_NT_HEADERS64, IMAGE_NT_OPTIONAL_HDR64_MAGIC,
        IMAGE_RUNTIME_FUNCTION_ENTRY, IMAGE_SECTION_HEADER,
    },
    SystemServices::{IMAGE_DOS_HEADER, IMAGE_DOS_SIGNATURE, IMAGE_NT_SIGNATURE},
};

use crate::pe::addr;

#[derive(Debug)]
pub struct Section {
    pub name: String,
    pub len: usize,
    pub start: addr::Va,
}

impl Section {
    pub fn contains(&self, cursor: addr::Va) -> bool {
        cursor >= self.start && cursor < self.start + self.len
    }
}

// SAFETY: the module stays loaded, we can give the static lifetime
// TODO: use PeView from pelite instead
pub fn get_nt_header(base: addr::Va) -> Result<&'static IMAGE_NT_HEADERS64> {
    let dos_header = unsafe { &*(base.as_ptr::<IMAGE_DOS_HEADER>()) };
    if dos_header.e_magic != IMAGE_DOS_SIGNATURE {
        bail!("invalid DOS signature at {:#x}", base);
    }

    let nt_headers = unsafe {
        &*((base + addr::Rva(dos_header.e_lfanew as u32)).as_ptr::<IMAGE_NT_HEADERS64>())
    };
    if nt_headers.Signature != IMAGE_NT_SIGNATURE {
        bail!("invalid PE signature at {:#x}", base);
    }

    // expect P32+
    if nt_headers.OptionalHeader.Magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC {
        bail!("invalid 64bits at {:#x}", base);
    }

    Ok(nt_headers)
}

pub fn sections(base: addr::Va) -> Result<Vec<Section>> {
    let nt = get_nt_header(base)?;
    let count = nt.FileHeader.NumberOfSections as usize;
    let opt_size = nt.FileHeader.SizeOfOptionalHeader as usize;
    let first =
        unsafe { (&raw const nt.OptionalHeader).byte_add(opt_size) } as *const IMAGE_SECTION_HEADER;

    let headers = unsafe { slice::from_raw_parts(first, count) };
    Ok(headers
        .iter()
        .map(|hdr| {
            let name_end = hdr.Name.iter().position(|&b| b == 0).unwrap_or(8);
            Section {
                name: String::from_utf8_lossy(&hdr.Name[..name_end]).into_owned(),
                len: unsafe { hdr.Misc.VirtualSize } as usize,
                start: base + addr::Rva(hdr.VirtualAddress),
            }
        })
        .collect())
}

pub fn find_section<'a>(secs: &'a [Section], name: &str) -> Result<&'a Section> {
    secs.iter()
        .find(|s| s.name == name)
        .with_context(|| format!("section {name} not found"))
}

pub fn pdata_starts(base: addr::Va) -> Result<Vec<Range<addr::Va>>> {
    let nt = get_nt_header(base)?;
    let data_dir = nt.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXCEPTION.0 as usize];
    let rva = addr::Rva(data_dir.VirtualAddress);
    let size = data_dir.Size as usize;

    let count = size / size_of::<IMAGE_RUNTIME_FUNCTION_ENTRY>();
    unsafe {
        Ok(
            slice::from_raw_parts((base + rva).as_ptr::<IMAGE_RUNTIME_FUNCTION_ENTRY>(), count)
                .iter()
                .map(|entry| {
                    Range::from(
                        base + addr::Rva(entry.BeginAddress)..base + addr::Rva(entry.EndAddress),
                    )
                })
                .collect(),
        )
    }
}
