// arceos-main/src/main.rs

#![no_std]
#![no_main]

use axstd::println;

extern crate axlinux;
extern crate axns;

/// This is the main function for the unikernel application.
/// It must be named `main` and have the C ABI to be called by `axruntime`.
#[unsafe(no_mangle)]
pub extern "C" fn main() {
    println!("[arceos-main] Application 'main' function started!");
    // ...
    println!("[arceos-main] Application 'main' function finished.");
}
