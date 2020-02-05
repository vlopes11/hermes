#[derive(Debug, Copy, Clone)]
pub struct Task {
    ptr: *mut u8,
    len: usize,
    cap: usize,
}

impl From<Vec<u8>> for Task {
    fn from(v: Vec<u8>) -> Self {
        let (ptr, len, cap) = v.into_raw_parts();
        Self { ptr, len, cap }
    }
}

impl Into<Vec<u8>> for Task {
    fn into(self) -> Vec<u8> {
        unsafe { Vec::from_raw_parts(self.ptr, self.len, self.cap) }
    }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}
