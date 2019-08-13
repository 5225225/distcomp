#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use wasmlib;

use wasmlib::output;

#[export_name = "main"]
fn main() {
    let mut fooVec = Vec::new();

    fooVec.push(1);
    fooVec.push(2);
    fooVec.push(3);

    for val in fooVec {
        output(&alloc::fmt::format(format_args!("got {} from vec!\n", val)));
    }
}
