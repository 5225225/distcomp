#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;

use wasmlib::prelude::*;

#[export_name = "main"]
fn main() {
    let state = get_state();

    let mut ctr = 10;

    if let Some(state) = state {
        println!("Got state with some key! (debug is {:x?}", state);

        let gotten = cas_get(&state);

        println!("Got {:x?} from the CAS!", gotten);

        println!("Setting ctr to {}", gotten[0] + 1);

        ctr = gotten[0] + 1;
    } else {
        println!("No state!");
    }

    let new_state = cas_put(&[ctr]);

    println!("Got state {:x?} as a CAS for ctr", new_state);

    update_state(&new_state);

    println!("Just wrote that state back. Run me again!");
}
