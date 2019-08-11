#![no_main]

//use wasmlib::{cas_get, cas_put, get_state, update_state};
use wasmlib::hello_world;

#[export_name = "main"]
fn main() {
    unsafe {
        hello_world();
    }
}
