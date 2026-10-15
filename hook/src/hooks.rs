use anyhow::{Context, Result, bail};
use retour::GenericDetour;
use seq_macro::seq;
use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{OnceLock, PoisonError};

use crate::symbols;

type EventFn = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize);

type Prefix = Box<dyn Fn(*mut c_void, *mut c_void) -> Option<bool> + Send + Sync>;
type Postfix = Box<dyn Fn(*mut c_void, *mut c_void) + Send + Sync>;

static SLOTS: [Slot; 64] = [const { Slot::new() }; 64];
static DISPATCH: [EventFn; 64] = seq!(N in 0..64 { [ #(dispatch::<N> as EventFn,)* ] });
static NEXT: AtomicUsize = AtomicUsize::new(0);

pub struct Slot {
    detour: OnceLock<GenericDetour<EventFn>>,
    prefixes: RwLock<Vec<Prefix>>,
    postfixes: RwLock<Vec<Postfix>>,
}

impl Slot {
    pub const fn new() -> Self {
        Slot {
            detour: OnceLock::new(),
            prefixes: RwLock::new(vec![]),
            postfixes: RwLock::new(vec![]),
        }
    }

    pub fn add_prefix(&self, prefix: Prefix) {
        self.prefixes
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .push(prefix);
    }

    pub fn add_postfix(&self, postfix: Postfix) {
        self.postfixes
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .push(postfix);
    }
}

impl Default for Slot {
    fn default() -> Self {
        Self::new()
    }
}

extern "C" fn dispatch<const N: usize>(
    this: *mut c_void,
    other: *mut c_void,
    a3: usize,
    a4: usize,
) {
    let s = &SLOTS[N];
    let run = std::panic::catch_unwind(AssertUnwindSafe(|| {
        s.prefixes
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .fold(true, |ok, prefix| prefix(this, other).unwrap_or(true) && ok)
    }))
    .unwrap_or(true);

    if run {
        unsafe {
            if let Some(f) = s.detour.get() {
                f.call(this, other, a3, a4)
            };
        }
    }

    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        s.postfixes
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .for_each(|postfix| postfix(this, other))
    }));
}

/// # Safety
///
/// The named function must have the EventFn signature
// TODO: make this function compatible with all gml_ functions
pub unsafe fn install(func_name: &str) -> Result<&'static Slot> {
    // retrieve the target adr before burning a slot
    let func = symbols::get_function_address(func_name)
        .with_context(|| format!("Cant find function {}", func_name))?;

    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    if n >= DISPATCH.len() {
        bail!("no free hook slot")
    }

    let target: EventFn = unsafe { std::mem::transmute::<usize, EventFn>(func.start.into()) };
    let d = unsafe { GenericDetour::new(target, DISPATCH[n]) }?;

    let slot = &SLOTS[n];
    if slot.detour.set(d).is_err() {
        bail!(
            "Detour of slot {} was already set making it invalid for later patches",
            n
        )
    }
    unsafe {
        slot.detour
            .get()
            .with_context(|| format!("Cannot get detour of slot {}", n))?
            .enable()
            .with_context(|| format!("Cannot enable detour of slot {}", n))?
    };
    Ok(slot)
}
