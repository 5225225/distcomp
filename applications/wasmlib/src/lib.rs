#![no_std]

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Key([u64; 4]);

extern "C" {
    /*
    pub fn update_state(key: Key);
    pub fn get_state() -> Key;

    pub fn cas_get(key: Key, len: usize, offset: usize, dest: *mut u8) -> usize;
    pub fn cas_put(len: usize, src: *mut u8) -> Key;
    */

    pub fn hello_world();
}
