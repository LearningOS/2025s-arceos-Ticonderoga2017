#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

use axstd::print_with_color;
#[cfg(feature = "axstd")]
use axstd::println;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // println!("[WithColor]: Hello, Arceos!");
    print_with_color!("blue", "[WithColor]: Hello, Arceos!");
}
