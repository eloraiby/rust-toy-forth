/*
#![feature(alloc_system)]
#![feature(global_allocator, allocator_api)]

extern crate alloc_system;
use alloc_system::System;

#[global_allocator]
static A: System = System;
*/
mod stream;
mod vm;
mod forth_core;

use std::fs::File;
use std::env;
use stream::StreamReader;
use vm::VM;
use forth_core::*;

fn main() {
    let args = env::args();
    
    let mut vm  = VM::new();

    for (i, ref a) in args.enumerate().skip(1) {
        println!("stream {}: {}", i, a);
        match File::open(a) {
            Err(e) => panic!("File {} not found: {}", a, e),
            Ok(file) => vm.add_stream(StreamReader::from_file(file))
        }
    }
    register(&mut vm);
    vm.add_stream(StreamReader::from_stdin());
    vm.repl();
}
