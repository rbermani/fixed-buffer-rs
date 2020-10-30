/// Program to compare heap memory usage of
/// `Box<FixedBuf<[u8; 256]>>` and `FixedBuf<Box<[u8; 256]>>`.
use std::println;

use clap::arg_enum;
use fixed_buffer::FixedBuf;
use jemallocator;

arg_enum! {
    #[derive(Debug)]
    enum BufType {
        BoxFixedBuf,
        FixedBufBox
    }
}

#[derive(structopt::StructOpt, Debug)]
struct Opt {
    #[structopt(possible_values = &BufType::variants(), case_insensitive = true)]
    buf_type: BufType,
}

fn print_active_mem() {
    jemalloc_ctl::epoch::mib().unwrap().advance().unwrap();
    println!(
        "{:.1} MiB active memory",
        (jemalloc_ctl::stats::active::read().unwrap() as f64) / (1024.0 * 1024.0)
    );
}

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() {
    let opt: Opt = structopt::StructOpt::from_args();
    match opt.buf_type {
        BufType::BoxFixedBuf => {
            println!("Making 1M Box<FixedBuf<[u8; 256]>> structs");
            let mut v: Vec<Box<FixedBuf<[u8; 256]>>> = Vec::new();
            for _ in 0..(1024 * 1024) {
                v.push(Box::new(FixedBuf::new([0; 256])));
            }
            print_active_mem();
        }
        BufType::FixedBufBox => {
            println!("Making 1M FixedBuf<Box<[u8; 256]>> structs");
            let mut v: Vec<FixedBuf<Box<[u8]>>> = Vec::new();
            for _ in 0..(1024 * 1024) {
                v.push(FixedBuf::new(Box::new([0; 256])));
            }
            print_active_mem();
        }
    }
}

// $ cargo run --package fixed-buffer --example box_benchmark BoxFixedBuf
// Making 1M Box<FixedBuf<[u8; 256]>> structs
// 328.1 MiB active memory
// $ cargo run --package fixed-buffer --example box_benchmark FixedBufBox
// Making 1M FixedBuf<Box<[u8; 256]>> structs
// 288.1 MiB active memory
