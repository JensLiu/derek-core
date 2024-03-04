// Copyright (c) 2020 Alex Chi
// 
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

#![no_std]
#![no_main]
#![feature(format_args_nl)]

use user::println;
use user::syscall::exit;

#[no_mangle]
pub unsafe extern "C" fn _start() -> ! {
    println!("test3!");
    exit(0);
}
