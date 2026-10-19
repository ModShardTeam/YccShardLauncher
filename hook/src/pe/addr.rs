use std::{
    fmt::LowerHex,
    ops::{Add, AddAssign, Sub},
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
