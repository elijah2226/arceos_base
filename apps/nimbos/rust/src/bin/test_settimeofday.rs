// Example: test_settimeofday.rs
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::{settimeofday, gettimeofday, TimeSpec};
use core::ptr::{null, null_mut};

#[unsafe(no_mangle)]
fn main() -> i32 {
    // Set time to a known value
    let tv = TimeSpec {
        sec: 123456789, // Example timestamp
        nsec: 0,
    };
    let ret = settimeofday(&tv, null());
    assert_eq!(ret, 0, "settimeofday failed");

    // Get time and check value
    let mut tv2 = TimeSpec {
        sec: 0,
        nsec: 0,
    };
    let ret2 = gettimeofday(&mut tv2, null_mut());
    assert_eq!(ret2, 0, "gettimeofday failed");
    println!("tv_sec: {}", tv2.sec);
    assert_eq!(tv2.sec, 123456789, "tv_sec does not match");

    println!("test_settimeofday passed!");
    0
}