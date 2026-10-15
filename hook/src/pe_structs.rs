use std::{
    fmt::LowerHex,
    ops::{Add, AddAssign, Range, Sub},
};

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Rva(pub u32);

impl std::fmt::Debug for Rva {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Rva")
            .field(&format!("{:#x}", &self.0))
            .finish()
    }
}

impl std::fmt::Display for Rva {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl LowerHex for Rva {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl Add<u32> for Rva {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Rva(self.0 + rhs)
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Va(pub usize);

impl Va {
    pub fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    // TODO: use a better abstraction
    pub fn rva_from_base(self, base: Va) -> Option<Rva> {
        let v = self.0.checked_sub(base.0)?;
        Some(Rva(u32::try_from(v).ok()?))
    }
}

impl std::fmt::Debug for Va {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Va")
            .field(&format!("{:#x}", &self.0))
            .finish()
    }
}

impl LowerHex for Va {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl Add<Rva> for Va {
    type Output = Self;

    fn add(self, rhs: Rva) -> Self::Output {
        Va(self.0 + rhs.0 as usize)
    }
}

impl Add<usize> for Va {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Va(self.0 + rhs)
    }
}

impl AddAssign<usize> for Va {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl Sub for Va {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl From<Va> for usize {
    fn from(value: Va) -> Self {
        value.0
    }
}

#[derive(Debug)]
pub struct Section {
    pub name: String,
    pub len: usize,
    pub start: Va,
}

pub type VarAddress = Va;

#[derive(Debug, Clone)]
pub struct ResolvedVar {
    pub name: &'static str,
    pub id_ptr: VarAddress,
}

pub type FnAddress = Range<Va>;

#[derive(Debug, Clone)]
pub struct ResolvedFn {
    pub name: &'static str,
    pub func_ptr: FnAddress,
}

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
