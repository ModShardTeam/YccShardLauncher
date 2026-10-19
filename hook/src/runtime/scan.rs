use std::cmp::min;
use std::collections::HashMap;
use std::mem::offset_of;
use std::slice;

use anyhow::Result;
use windows::Win32::System::Diagnostics::Debug::{
    IMAGE_DIRECTORY_ENTRY_EXCEPTION, IMAGE_RUNTIME_FUNCTION_ENTRY,
};

use crate::pe::{addr, image};
use crate::runtime::{layout, symbols};

#[derive(Debug)]
pub enum NameRejection {
    OutOfBounds,
    NoTerminator,
    Empty,
    NotIdentifier,
}

#[derive(Debug)]
pub enum VarEntryRejection {
    BadId,
    NonZeroPad,
    BadName(NameRejection),
}

impl From<NameRejection> for VarEntryRejection {
    fn from(n: NameRejection) -> Self {
        VarEntryRejection::BadName(n)
    }
}

#[derive(Debug)]
pub enum FnEntryRejection {
    FuncNotInText,
    SlotNotInData,
    BadPrefix,
    IncompatibleSlotName,
    NotFoundInPdata,
    BadName(NameRejection),
}

impl From<NameRejection> for FnEntryRejection {
    fn from(n: NameRejection) -> Self {
        FnEntryRejection::BadName(n)
    }
}

#[derive(Debug, Default)]
pub struct NameStats {
    pub out_of_bounds_count: usize,
    pub no_terminator_count: usize,
    pub empty_count: usize,
    pub not_identifier_count: usize,
}

impl NameStats {
    pub fn record(&mut self, rej: &NameRejection) {
        match rej {
            NameRejection::OutOfBounds => self.out_of_bounds_count += 1,
            NameRejection::NoTerminator => self.no_terminator_count += 1,
            NameRejection::Empty => self.empty_count += 1,
            NameRejection::NotIdentifier => self.not_identifier_count += 1,
        }
    }
}

#[derive(Debug, Default)]
pub struct VarStats {
    pub bad_id_count: usize,
    pub non_zero_pad_count: usize,
    pub name: NameStats,
}

impl VarStats {
    pub fn record(&mut self, rej: &VarEntryRejection) {
        match rej {
            VarEntryRejection::BadId => self.bad_id_count += 1,
            VarEntryRejection::NonZeroPad => self.non_zero_pad_count += 1,
            VarEntryRejection::BadName(r) => {
                self.name.record(r);
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct FnStats {
    pub func_not_in_text_count: usize,
    pub slot_not_in_data_count: usize,
    pub bad_prefix_count: usize,
    pub incompatible_slot_name_count: usize,
    pub not_found_in_pdata_count: usize,
    pub name: NameStats,
}

impl FnStats {
    pub fn record(&mut self, rej: &FnEntryRejection) {
        match rej {
            FnEntryRejection::FuncNotInText => self.func_not_in_text_count += 1,
            FnEntryRejection::SlotNotInData => self.slot_not_in_data_count += 1,
            FnEntryRejection::BadPrefix => self.bad_prefix_count += 1,
            FnEntryRejection::IncompatibleSlotName => self.incompatible_slot_name_count += 1,
            FnEntryRejection::NotFoundInPdata => self.not_found_in_pdata_count += 1,
            FnEntryRejection::BadName(r) => {
                self.name.record(r);
            }
        }
    }
}

fn pdata_starts(base: addr::Va) -> Result<Vec<symbols::FnAddress>> {
    let nt = image::get_nt_header(base)?;
    let data_dir = nt.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXCEPTION.0 as usize];
    let rva = addr::Rva(data_dir.VirtualAddress);
    let size = data_dir.Size as usize;

    let count = size / size_of::<IMAGE_RUNTIME_FUNCTION_ENTRY>();
    unsafe {
        Ok(
            slice::from_raw_parts((base + rva).as_ptr::<IMAGE_RUNTIME_FUNCTION_ENTRY>(), count)
                .iter()
                .map(|entry| symbols::FnAddress {
                    start: base + addr::Rva(entry.BeginAddress),
                    end: base + addr::Rva(entry.EndAddress),
                })
                .collect(),
        )
    }
}

// SAFETY: the module stays loaded, we can give the static lifetime
fn read_name(rdata: &image::Section, ptr: addr::Va) -> Result<&'static str, NameRejection> {
    if !image::in_section(rdata, ptr) {
        return Err(NameRejection::OutOfBounds);
    }
    let max = min(rdata.start + rdata.len - ptr, layout::MAX_SIZE_GML_NAME);
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

fn var_entry_at(
    rdata: &image::Section,
    cursor: addr::Va,
) -> Result<symbols::ResolvedVar, VarEntryRejection> {
    // cursor is guaranteed to be inside .data
    unsafe {
        let entry: layout::YYCVar = *(cursor.as_ptr::<layout::YYCVar>());
        // id is -1 before game initialization
        // id > 0 after
        if entry.id != -1 && entry.id <= 0 {
            return Err(VarEntryRejection::BadId);
        }
        // there is a zero padding after the id
        if entry.zero != 0 {
            return Err(VarEntryRejection::NonZeroPad);
        }

        Ok(symbols::ResolvedVar {
            name: read_name(rdata, addr::Va(entry.name_ptr))?,
            id_ptr: cursor + offset_of!(layout::YYCVar, id),
        })
    }
}

fn fn_entry_at(
    data: &image::Section,
    rdata: &image::Section,
    text: &image::Section,
    starts: &[symbols::FnAddress],
    cursor: addr::Va,
) -> Result<symbols::ResolvedFn, FnEntryRejection> {
    // cursor is guaranteed to be inside .data
    unsafe {
        let entry = *(cursor.as_ptr::<layout::YYCFn>());
        if !image::in_section(text, addr::Va(entry.func)) {
            return Err(FnEntryRejection::FuncNotInText);
        }
        if !image::in_section(data, addr::Va(entry.slot_ptr)) {
            return Err(FnEntryRejection::SlotNotInData);
        }

        let name = read_name(rdata, addr::Va(entry.name_ptr))?;

        if !layout::GML_PREFIXES.iter().any(|pre| name.starts_with(pre)) {
            return Err(FnEntryRejection::BadPrefix);
        }
        // TODO: this read is unaligned
        if *(entry.slot_ptr as *const usize) != entry.name_ptr {
            return Err(FnEntryRejection::IncompatibleSlotName);
        }

        let Ok(i) = starts.binary_search_by_key(&addr::Va(entry.func), |f| f.start) else {
            return Err(FnEntryRejection::NotFoundInPdata);
        };

        Ok(symbols::ResolvedFn {
            name,
            func_ptr: starts[i],
        })
    }
}

pub fn scan_var_table(base: addr::Va) -> Result<HashMap<String, symbols::VarAddress>> {
    let mut stats = VarStats::default();
    let secs = image::sections(base)?;
    let rdata = image::find_section(&secs, ".rdata")?;
    let data = image::find_section(&secs, ".data")?;

    let mut best: Vec<symbols::ResolvedVar> = vec![];
    let mut best_rej: Option<VarEntryRejection> = None;
    let mut off = 0usize;
    while off + size_of::<layout::YYCVar>() <= data.len {
        let mut run: Vec<symbols::ResolvedVar> = vec![];
        let mut run_rej: Option<VarEntryRejection> = None;
        let mut cursor = data.start + off;
        while cursor + size_of::<layout::YYCVar>() <= data.start + data.len {
            match var_entry_at(rdata, cursor) {
                Ok(entry) => {
                    run.push(entry);
                    cursor += size_of::<layout::YYCVar>();
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
            off + run.len() * size_of::<layout::YYCVar>() + 8
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

pub fn scan_fn_table(base: addr::Va) -> Result<HashMap<String, symbols::FnAddress>> {
    let mut stats = FnStats::default();
    let secs = image::sections(base)?;
    let rdata = image::find_section(&secs, ".rdata")?;
    let data = image::find_section(&secs, ".data")?;
    let text = image::find_section(&secs, ".text")?;
    let starts = pdata_starts(base)?;

    let mut map = HashMap::new();

    let mut cursor = data.start;
    while cursor + size_of::<layout::YYCFn>() <= data.start + data.len {
        match fn_entry_at(data, rdata, text, &starts, cursor) {
            Ok(entry) => {
                map.insert(entry.name.to_string(), entry.func_ptr);
                cursor += size_of::<layout::YYCFn>();
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
