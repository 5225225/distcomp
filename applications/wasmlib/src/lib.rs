#![feature(never_type)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![no_std]

use core::num::NonZeroU32;

#[repr(transparent)]
#[derive(Debug)]
pub struct KeyHandle(NonZeroU32);

#[repr(transparent)]
#[derive(Debug)]
pub struct CASHandle(NonZeroU32);

pub mod cas_referenced;
pub mod stack;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern crate alloc;

extern "C" {
    // Updates the current state to that in key

    #[link_name = "update_state"]
    fn _update_state(key: u32);

    // Writes the current state into the pointer in key. Key does not need to be init.
    // returns non-zero on error (update_state has never been called for this application?)
    #[link_name = "get_state"]
    fn _get_state() -> u32;

    // Gets the data from key, starting at offset bytes in the blob, writing at most len bytes to dest.
    // Returns actual number of bytes read.
    // On error (key invalid?) returns negative
    #[link_name = "cas_get"]
    fn _cas_get(key: u32) -> u32;

    // starting at src, read len bytes, and insert it as an object, writing into key (key does not
    // need to be init)
    #[link_name = "cas_put"]
    fn _cas_put(src: *const u8, len: usize) -> u32;

    // Writes len bytes to the screen, starting at src. Returns number of bytes written.
    #[link_name = "output"]
    fn _output(src: *const u8, len: usize) -> usize;
}

pub fn update_state(key: &KeyHandle) {
    unsafe {
       _update_state(key.0.get());
    }
}

pub fn get_state() -> Option<KeyHandle> {
    unsafe {
        Some(KeyHandle(NonZeroU32::new(_get_state())?))
    }
}

const BUF_SIZE: usize = 1 << 16;

pub fn cas_get_into(key: &Key, buf: &mut alloc::vec::Vec<u8>) {
    /*
    let mut offset = 0;
    loop {
        let mut in_buf = [0; BUF_SIZE];
        let size;
        unsafe {
            size = _cas_get(key, offset, BUF_SIZE, in_buf.as_mut_ptr());
        }
        if size == 0 {
            break;
        }
        if size < 0 {
            panic!("Some error happened while reading");
        }
        offset += size as usize;
        buf.extend_from_slice(&in_buf[0..size as usize]);
    }
    */
}

pub fn cas_get(key: &KeyHandle) -> Option<CASHandle> {
    let h;
    unsafe {
        h = _cas_get(key.0.get());
    }

    Some(CASHandle(NonZeroU32::new(h)?))
}

pub fn cas_put(data: &[u8]) -> Option<KeyHandle> {
    let k;

    unsafe {
        k = _cas_put(data.as_ptr(), data.len());
    }

    Some(KeyHandle(NonZeroU32::new(k)?))
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        $crate::_print_nl(format_args!($($arg)*));
    })
}

pub fn _print(args: core::fmt::Arguments) {
    core::fmt::write(&mut Output(), args).unwrap();
}

pub fn _print_nl(args: core::fmt::Arguments) {
    core::fmt::write(&mut Output(), args).unwrap();

    core::fmt::write(&mut Output(), format_args!("\n")).unwrap();
}

pub struct Output();

impl core::fmt::Write for Output {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        output(s);

        Ok(())
    }
}

pub fn output(s: &str) {
    unsafe {
        _output(s.as_ptr(), s.len());
    }
}

#[panic_handler]
fn panic(panic: &core::panic::PanicInfo) -> ! {
    print!("panic at "); // the disco

    if let Some(location) = panic.location() {
        print!(
            "file {} line {} column {} ",
            location.file(),
            location.line(),
            location.column()
        )
    } else {
        print!("an unknown location ")
    }

    if let Some(message) = panic.message() {
        let _ = core::fmt::write(&mut Output(), *message);
    }

    print!("\n\n");

    loop {}
}

#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    print!("failed to allocate {:?}", layout);

    loop {}
}

pub mod prelude {
    pub use crate::print;
    pub use crate::println;

    pub use crate::cas_get;
    pub use crate::cas_put;
    pub use crate::get_state;
    pub use crate::update_state;

    pub use crate::stack::Stack;

    pub use crate::cas_referenced::CASReferenced;
}
