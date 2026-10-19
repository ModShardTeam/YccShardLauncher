use std::{cell::Cell, ffi::c_void};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub this: *mut c_void,
    pub other: *mut c_void,
}

struct RestoredFrame(Option<Frame>);

impl Drop for RestoredFrame {
    fn drop(&mut self) {
        CURRENT.set(self.0);
    }
}

thread_local! {
    static CURRENT: Cell<Option<Frame>> = const { Cell::new(None) };
}

pub fn with_frame<R>(frame: Frame, f: impl FnOnce() -> R) -> R {
    let _to_be_restored_frame = RestoredFrame(CURRENT.replace(Some(frame)));
    f()
}

pub fn current() -> Option<Frame> {
    CURRENT.get()
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, panic::catch_unwind};

    use super::*;
    fn create_fake_cinstance() -> *mut c_void {
        Box::into_raw(Box::new(HashMap::<String, f64>::new())) as *mut c_void
    }
    fn create_fake_frame() -> Frame {
        Frame {
            this: create_fake_cinstance(),
            other: create_fake_cinstance(),
        }
    }

    #[test]
    fn current_called_outside_frame() {
        assert_eq!(current(), None);
    }

    #[test]
    fn current_called_inside_frame() {
        let a = create_fake_frame();
        with_frame(a, || assert_eq!(current(), Some(a)));
        assert_eq!(current(), None);
    }

    #[test]
    fn nested_frames() {
        let a = create_fake_frame();
        let b = create_fake_frame();
        with_frame(a, || {
            assert_eq!(current(), Some(a));
            with_frame(b, || assert_eq!(current(), Some(b)));
            assert_eq!(current(), Some(a))
        });
        assert_eq!(current(), None);
    }

    #[test]
    fn restores_outer_frame_after_inner_panic() {
        let a = create_fake_frame();
        let b = create_fake_frame();
        with_frame(a, || {
            assert_eq!(current(), Some(a));
            let after_panic = catch_unwind(|| {
                with_frame(b, || {
                    assert_eq!(current(), Some(b));
                    panic!("expected panic")
                });
            });
            let err = after_panic.expect_err("inner frame should have panicked");
            assert_eq!(err.downcast_ref::<&str>(), Some(&"expected panic"));
            assert_eq!(current(), Some(a));
        });

        assert_eq!(current(), None);
    }
}
