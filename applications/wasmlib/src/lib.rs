#![feature(alloc_error_handler)]
#![no_std]

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Key([u64; 4]);

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern "C" {
    // Updates the current state to that in key

    #[link_name = "update_state"]
    fn _update_state(key: *const Key);

    // Writes the current state into the pointer in key. Key does not need to be init.
    // returns non-zero on error (update_state has never been called for this application?)
    #[link_name = "get_state"]
    fn _get_state(key: *mut Key) -> i32;

    // Gets the data from key, starting at offset bytes in the blob, writing at most len bytes to dest.
    // Returns actual number of bytes read.
    // On error (key invalid?) returns negative
    #[link_name = "cas_get"]
    fn _cas_get(key: *const Key, offset: usize, len: usize, dest: *mut u8) -> i64;

    // starting at src, read len bytes, and insert it as an object, writing into key (key does not
    // need to be init)
    #[link_name = "cas_put"]
    fn _cas_put(src: *const u8, len: usize, key: *mut Key);

    // Writes len bytes to the screen, starting at src. Returns number of bytes written.
    #[link_name = "output"]
    fn _output(src: *const u8, len: usize) -> usize;
}

pub fn update_state(key: &Key) {
    unsafe {
        _update_state(key);
    }
}

pub fn get_state() -> Option<Key> {
    let mut key = core::mem::MaybeUninit::<Key>::uninit();

    unsafe {
        let ret = _get_state(key.as_mut_ptr());

        if ret == 0 {
            Some(key.assume_init())
        } else {
            None
        }
    }
}

pub fn output(s: &str) {
    unsafe {
        _output(s.as_ptr(), s.len());
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    loop {}
}
