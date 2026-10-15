use crate::{
    pe_structs::{
        FnAddress, FnEntryRejection, FnStats, NameRejection, ResolvedFn, ResolvedVar, Rva, Section,
        Va, VarAddress, VarEntryRejection, VarStats,
    },
    yyc_structs::{YYCFn, YYCVar},
};
use anyhow::{Context, Result, bail};
use std::{cmp::min, collections::HashMap, mem::offset_of, slice};
use windows::Win32::System::{
    Diagnostics::Debug::{
        IMAGE_DIRECTORY_ENTRY_EXCEPTION, IMAGE_NT_HEADERS64, IMAGE_NT_OPTIONAL_HDR64_MAGIC,
        IMAGE_RUNTIME_FUNCTION_ENTRY, IMAGE_SECTION_HEADER,
    },
    SystemServices::{IMAGE_DOS_HEADER, IMAGE_DOS_SIGNATURE, IMAGE_NT_SIGNATURE},
};

const MAX_SIZE_GML_NAME: usize = 128;
const GML_PREFIXES: [&str; 4] = [
    "gml_Object_",
    "gml_Script_",
    "gml_GlobalScript_",
    "gml_RoomCC_",
];

// SAFETY: the module stays loaded, we can give the static lifetime
// TODO: use PeView from pelite instead
fn get_nt_header(base: Va) -> Result<&'static IMAGE_NT_HEADERS64> {
    let dos_header = unsafe { &*(base.as_ptr::<IMAGE_DOS_HEADER>()) };
    if dos_header.e_magic != IMAGE_DOS_SIGNATURE {
        bail!("invalid DOS signature at {:#x}", base);
    }

    let nt_headers =
        unsafe { &*((base + Rva(dos_header.e_lfanew as u32)).as_ptr::<IMAGE_NT_HEADERS64>()) };
    if nt_headers.Signature != IMAGE_NT_SIGNATURE {
        bail!("invalid PE signature at {:#x}", base);
    }

    // expect P32+
    if nt_headers.OptionalHeader.Magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC {
        bail!("invalid 64bits at {:#x}", base);
    }

    Ok(nt_headers)
}

fn sections(base: Va) -> Result<Vec<Section>> {
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
                start: base + Rva(hdr.VirtualAddress),
            }
        })
        .collect())
}

fn pdata_starts(base: Va) -> Result<Vec<FnAddress>> {
    let nt = get_nt_header(base)?;
    let data_dir = nt.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXCEPTION.0 as usize];
    let rva = Rva(data_dir.VirtualAddress);
    let size = data_dir.Size as usize;

    let count = size / size_of::<IMAGE_RUNTIME_FUNCTION_ENTRY>();
    unsafe {
        Ok(
            slice::from_raw_parts((base + rva).as_ptr::<IMAGE_RUNTIME_FUNCTION_ENTRY>(), count)
                .iter()
                .map(|entry| FnAddress {
                    start: base + Rva(entry.BeginAddress),
                    end: base + Rva(entry.EndAddress),
                })
                .collect(),
        )
    }
}

fn in_section(s: &Section, cursor: Va) -> bool {
    cursor >= s.start && cursor < s.start + s.len
}

fn find_section<'a>(secs: &'a [Section], name: &str) -> Result<&'a Section> {
    secs.iter()
        .find(|s| s.name == name)
        .with_context(|| format!("section {name} not found"))
}

// SAFETY: the module stays loaded, we can give the static lifetime
fn read_name(rdata: &Section, ptr: Va) -> Result<&'static str, NameRejection> {
    if !in_section(rdata, ptr) {
        return Err(NameRejection::OutOfBounds);
    }
    let max = min(rdata.start + rdata.len - ptr, MAX_SIZE_GML_NAME);
    let bytes = unsafe { slice::from_raw_parts(ptr.as_ptr::<u8>(), max) };
    let end = bytes
        .iter()
        .position(|&b| b == 0)
        .ok_or(NameRejection::NoTerminator)?;
    if end == 0 {
        return Err(NameRejection::Empty);
    }
    if !bytes[..end]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        return Err(NameRejection::NotIdentifier);
    }
    unsafe {
        // SAFETY: we already check &bytes[..end] contains only ASCII chars
        Ok(std::str::from_utf8_unchecked(&bytes[..end]))
    }
}

fn var_entry_at(rdata: &Section, cursor: Va) -> Result<ResolvedVar, VarEntryRejection> {
    // cursor is guaranteed to be inside .data
    unsafe {
        let entry: YYCVar = *(cursor.as_ptr::<YYCVar>());
        // id is -1 before game initialization
        // id > 0 after
        if entry.id != -1 && entry.id <= 0 {
            return Err(VarEntryRejection::BadId);
        }
        // there is a zero padding after the id
        if entry.zero != 0 {
            return Err(VarEntryRejection::NonZeroPad);
        }

        Ok(ResolvedVar {
            name: read_name(rdata, Va(entry.name_ptr))?,
            id_ptr: cursor + offset_of!(YYCVar, id),
        })
    }
}

fn fn_entry_at(
    data: &Section,
    rdata: &Section,
    text: &Section,
    starts: &[FnAddress],
    cursor: Va,
) -> Result<ResolvedFn, FnEntryRejection> {
    // cursor is guaranteed to be inside .data
    unsafe {
        let entry = *(cursor.as_ptr::<YYCFn>());
        if !in_section(text, Va(entry.func)) {
            return Err(FnEntryRejection::FuncNotInText);
        }
        if !in_section(data, Va(entry.slot_ptr)) {
            return Err(FnEntryRejection::SlotNotInData);
        }

        let name = read_name(rdata, Va(entry.name_ptr))?;

        if !GML_PREFIXES.iter().any(|pre| name.starts_with(pre)) {
            return Err(FnEntryRejection::BadPrefix);
        }
        // TODO: this read is unaligned
        if *(entry.slot_ptr as *const usize) != entry.name_ptr {
            return Err(FnEntryRejection::IncompatibleSlotName);
        }

        let Ok(i) = starts.binary_search_by_key(&Va(entry.func), |f| f.start) else {
            return Err(FnEntryRejection::NotFoundInPdata);
        };

        Ok(ResolvedFn {
            name,
            func_ptr: starts[i].clone(),
        })
    }
}

pub fn scan_var_table(base: Va) -> Result<HashMap<String, VarAddress>> {
    let mut stats = VarStats::default();
    let secs = sections(base)?;
    let rdata = find_section(&secs, ".rdata")?;
    let data = find_section(&secs, ".data")?;

    let mut best: Vec<ResolvedVar> = vec![];
    let mut best_rej: Option<VarEntryRejection> = None;
    let mut off = 0usize;
    while off + size_of::<YYCVar>() <= data.len {
        let mut run: Vec<ResolvedVar> = vec![];
        let mut run_rej: Option<VarEntryRejection> = None;
        let mut cursor = data.start + off;
        while cursor + size_of::<YYCVar>() <= data.start + data.len {
            match var_entry_at(rdata, cursor) {
                Ok(entry) => {
                    run.push(entry);
                    cursor += size_of::<YYCVar>();
                }
                Err(e) => {
                    stats.record(&e);
                    run_rej = Some(e);
                    break;
                }
            }
        }
        off = if run.is_empty() {
            off + 8
        } else {
            off + run.len() * size_of::<YYCVar>() + 8
        };
        if run.len() > best.len() {
            best_rej = run_rej;
            best = run;
        }
    }
    log!("var table: {} entries", best.len());
    log!("rejection of the best run: {:#?}", best_rej);

    let mut map = HashMap::with_capacity(best.len());
    for entry in best {
        map.entry(entry.name.to_string()).or_insert(entry.id_ptr);
    }
    log!("total rejections: {:#?}", stats);
    Ok(map)
}

pub fn scan_fn_table(base: Va) -> Result<HashMap<String, FnAddress>> {
    let mut stats = FnStats::default();
    let secs = sections(base)?;
    let rdata = find_section(&secs, ".rdata")?;
    let data = find_section(&secs, ".data")?;
    let text = find_section(&secs, ".text")?;
    let starts = pdata_starts(base)?;

    let mut map = HashMap::new();

    let mut cursor = data.start;
    while cursor + size_of::<YYCFn>() <= data.start + data.len {
        match fn_entry_at(data, rdata, text, &starts, cursor) {
            Ok(entry) => {
                map.insert(entry.name.to_string(), entry.func_ptr);
                cursor += size_of::<YYCFn>();
            }
            Err(e) => {
                stats.record(&e);
                cursor += 8;
            }
        }
    }
    log!("fn table: {} entries", map.len());
    log!("total rejections: {:#?}", stats);
    Ok(map)
}
