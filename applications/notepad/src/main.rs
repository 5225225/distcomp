#![no_std]
#![no_main]

extern crate alloc;

use serde::{Deserialize, Serialize};
use wasmlib::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct MyState {
    ctr: i32,
    stack: Stack<()>,
}

#[export_name = "main"]
fn main() {
    let mut state;

    let get_state: Option<MyState> = get_state()
        .map(CASReferenced::from_key)
        .map(|x| x.get().expect("failed to deserialize data for state"));

    if let Some(get_state) = get_state {
        println!("Got some state {:?}", get_state);

        state = get_state;
    } else {
        println!("No state!");

        state = MyState {
            ctr: 0,
            stack: Stack::new(),
        };
    }

    println!("The state looks like {:?}", state);

    state.ctr += 1;

    println!("The state looks like {:?} after incrementing ctr", state);

    state.stack = state.stack.push(());

    let mut ctr = 0;

    state.stack.walk_backwards(&mut |_| {
        println!("counter: {}", ctr);
        ctr += 1;
    });

    println!(
        "The state looks like {:?} after pushing to the stack",
        state
    );

    let key = CASReferenced::put(state);

    println!("Going to update_state with key {:?}", key);

    update_state(&key.key);

    println!("Wrote state. Run me again!")
}
