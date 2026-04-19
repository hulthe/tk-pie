extern crate alloc;

use core::mem::MaybeUninit;

use embedded_alloc::LlffHeap;

pub const HEAP_SIZE: usize = 4096;

// Linked-List-First-Fit heap is supposedly better most of the time.
// https://github.com/rust-embedded/embedded-alloc/pull/78
#[global_allocator]
static HEAP: LlffHeap = LlffHeap::empty();

pub fn init() {
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
}
