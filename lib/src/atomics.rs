use atomic_polyfill::{AtomicU32, Ordering};

pub struct AtomicCoord {
    inner: AtomicU32,
}

impl AtomicCoord {
    pub const fn new() -> Self {
        AtomicCoord {
            inner: AtomicU32::new(0),
        }
    }

    pub fn load(&self, ordering: Ordering) -> (u16, u16) {
        let [a, b, c, d] = self.inner.load(ordering).to_ne_bytes();
        let x = u16::from_ne_bytes([a, b]);
        let y = u16::from_ne_bytes([c, d]);

        (x, y)
    }

    pub fn store(&self, x: u16, y: u16, ordering: Ordering) {
        let [a, b] = x.to_ne_bytes();
        let [c, d] = y.to_ne_bytes();
        let xy = u32::from_ne_bytes([a, b, c, d]);
        self.inner.store(xy, ordering);
    }
}
