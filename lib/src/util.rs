use core::{
    cell::UnsafeCell,
    fmt::{self, Debug, Display, Write},
    mem::MaybeUninit,
    ops::Deref,
};

use embassy_futures::yield_now;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Timer};
use msgpck::{Marker, MsgPack, PackErr, Piece};
use portable_atomic::{AtomicU32, Ordering};

use crate::rgb::Rgb;

pub type CS = CriticalSectionRawMutex;

#[derive()]
pub struct SwapCell<T> {
    // active_slot: 1 bit
    // readers: 31 bits
    entries: AtomicU32,
    exits: AtomicU32,

    slot0: UnsafeCell<MaybeUninit<T>>,
    slot1: UnsafeCell<MaybeUninit<T>>,
}

pub struct SwapCellWrite<T: 'static> {
    cell: &'static SwapCell<T>,
}

#[derive(Clone)]
pub struct SwapCellRead<T: 'static> {
    cell: &'static SwapCell<T>,
}

pub struct SwapCellGuard<'a, T> {
    cell: &'a SwapCell<T>,
    slot: &'a T,
}

const SLOT_MASK: u32 = 0x80000000;
const ENTRIES_MASK: u32 = 0x7FFFFFFF;

impl<T> SwapCell<T> {
    pub const fn new(initial: T) -> Self {
        SwapCell {
            entries: AtomicU32::new(0),
            exits: AtomicU32::new(0),
            slot0: UnsafeCell::new(MaybeUninit::new(initial)),
            slot1: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn split(&'static mut self) -> (SwapCellRead<T>, SwapCellWrite<T>) {
        (SwapCellRead { cell: self }, SwapCellWrite { cell: self })
    }
}

impl<T> SwapCellWrite<T> {
    pub async fn write(&mut self, t: T) {
        let x = self.cell.entries.load(Ordering::SeqCst);
        let active_slot = (x & SLOT_MASK).rotate_left(1) as usize;

        let mut slots = [&self.cell.slot0, &self.cell.slot1];
        slots.rotate_left(active_slot);
        let [active_slot, inactive_slot] = slots;

        // SAFETY: no one elase is allowed to touch the inactive slot.
        unsafe { inactive_slot.get().write(MaybeUninit::new(t)) };

        // swap active/inactive slots
        let x = self.cell.entries.fetch_xor(SLOT_MASK, Ordering::SeqCst);
        let (_active_slot, inactive_slot) = (inactive_slot, active_slot);

        // wait until there are no more readers in the previously active slot
        let entries = x & ENTRIES_MASK;
        loop {
            let exits = self.cell.exits.load(Ordering::SeqCst);
            if exits >= entries {
                break;
            }

            yield_now().await;
        }

        // drop the now inactive slot
        // SAFETY: we waited until everyone else stopped using this slot.
        unsafe { inactive_slot.get().drop_in_place() };
    }
}

impl<T> SwapCellRead<T> {
    pub fn read(&self) -> SwapCellGuard<'_, T> {
        let x = self.cell.entries.fetch_add(1, Ordering::SeqCst);

        let entries = x & ENTRIES_MASK;
        debug_assert!(entries < ENTRIES_MASK, "SwapCell overflowed");

        let slot = (x & SLOT_MASK).rotate_left(1) as usize;
        let slot = [&self.cell.slot0, &self.cell.slot1][slot];
        let slot = unsafe { &*slot.get() };
        let slot = unsafe { slot.assume_init_ref() };

        SwapCellGuard {
            cell: self.cell,
            slot,
        }
    }
}

impl<T> Drop for SwapCellGuard<'_, T> {
    fn drop(&mut self) {
        self.cell.exits.fetch_add(1, Ordering::SeqCst);
    }
}

impl<T> Deref for SwapCellGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.slot
    }
}

pub async fn stall() -> ! {
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Input a value 0 to 255 to get a color value
// The colours are a transition r - g - b - back to r.
pub fn wheel(mut wheel_pos: u8) -> Rgb {
    wheel_pos = 255 - wheel_pos;
    if wheel_pos < 85 {
        Rgb::new(255 - wheel_pos * 3, 0, wheel_pos * 3)
    } else if wheel_pos < 170 {
        wheel_pos -= 85;
        Rgb::new(0, wheel_pos * 3, 255 - wheel_pos * 3)
    } else {
        wheel_pos -= 170;
        Rgb::new(wheel_pos * 3, 255 - wheel_pos * 3, 0)
    }
}

/// Calculate the length of the formatted string of a type that impls Display.
pub fn display_len(t: &impl Display) -> usize {
    // impl fmt::Write for a dummy struct that just increments a length
    struct Write<'a>(&'a mut usize);
    impl fmt::Write for Write<'_> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            *self.0 += s.len();
            Ok(())
        }
    }

    let mut n = 0;

    let mut w = Write(&mut n);
    write!(&mut w, "{}", t).expect("Write impl is infallible");

    n
}

/// Wrapper type that impls [MsgPack] for `T: Display`, by serializing `T` as a msgpck string.
pub struct DisplayPack<T>(pub T);
impl<T: Display> MsgPack for DisplayPack<T> {
    fn pack(&self) -> impl Iterator<Item = msgpck::Piece<'_>> {
        "TODO".pack()
        //let len = display_len(&self.0) as u32;

        //[msgpck::Marker::Str32.into(), len.into()]
        //    .into_iter()
        //    .chain(iter::from_fn(move || None)) // TODO
    }

    fn pack_with_writer(&self, w: &mut dyn msgpck::Write) -> Result<usize, PackErr> {
        let mut n = 0;

        let str_len = display_len(&self.0);

        for bytes in [
            Piece::from(Marker::Str32).as_bytes(),
            Piece::from(str_len as u32).as_bytes(),
        ] {
            w.write_all(bytes)?;
            n += bytes.len();
        }

        // this is advanced stupid.
        struct Write<'a>(&'a mut dyn msgpck::Write);
        impl fmt::Write for Write<'_> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)?;
                Ok(())
            }
        }

        write!(&mut Write(w), "{}", self.0).map_err(|_| PackErr::BufferOverflow)?;
        n += str_len;

        Ok(n)
    }
}

impl<T: Debug> Debug for DisplayPack<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}
