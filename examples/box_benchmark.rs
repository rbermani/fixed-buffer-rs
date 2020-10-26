// Example server that uses `fixed_buffer` crate to parse a simple line-based protocol.

use std::println;

trait Buf {
    fn mem_mut(&mut self) -> &mut [u8];
    fn mem(&self) -> &[u8];
    fn field1(&self) -> bool;
    fn set_field1(&mut self, v: bool);

    fn modify_and_get_buf(&mut self) -> &[u8] {
        self.mem_mut()[1] = 'x' as u8;
        &self.mem()[..3]
    }

    fn debug(&self, name: &str, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{{field1 {:?}, mem \"{}\"}}",
            name,
            self.field1(),
            fixed_buffer::escape_ascii(self.mem())
        )
    }
}

macro_rules! generate_buf_n_structs {
    ( $( $NAME:ident, $LEN:expr )+ ) => {
        $(
            /// A Buf that contains its buffer memory.
            /// You can allocate this on the stack.
            ///
            /// Buf16 has 16 bytes.  Buf16KB has 16 KiB.
            pub struct $NAME {
                mem: [u8; $LEN],
                field1: bool,
            }
            impl $NAME {
                pub fn new() -> $NAME {
                    $NAME {
                        mem: ['.' as u8; $LEN],
                        field1: false,
                    }
                }
            }
            impl Buf for $NAME {
                fn mem_mut(&mut self) -> &mut [u8] {
                    &mut self.mem[..]
                }

                fn mem(&self) -> &[u8] {
                    &self.mem[..]
                }

                fn field1(&self) -> bool {
                    self.field1
                }

                fn set_field1(&mut self, v: bool) {
                    self.field1 = v;
                }
            }
            impl std::fmt::Debug for $NAME {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.debug(stringify!($NAME), f)
                }
            }
        )+
    }
}

generate_buf_n_structs! {
    Buf4, 4
    Buf8, 8
    Buf16, 16
    Buf32, 32
    Buf64, 64
    Buf100, 100
    Buf128, 128
    Buf200, 200
    Buf256, 256
    Buf512, 512
    Buf1KB, 1024
    Buf2KB, 2 * 1024
    Buf4KB, 4 * 1024
    Buf8KB, 8 * 1024
    Buf16KB, 16 * 1024
    Buf32KB, 32 * 1024
    Buf64KB, 64 * 1024
    Buf128KB, 128 * 1024
    Buf256KB, 256 * 1024
    Buf512KB, 512 * 1024
    Buf1MB, 1024 * 1024
}

macro_rules! generate_box_buf_n_structs {
    ( $( $NAME:ident, $LEN:expr )+ ) => {
        $(
            /// A Buf that allocates its buffer memory on the heap.
            ///
            /// Buf256 has 256 bytes.  Buf256KB has 256 KiB.
            ///
            /// Use BoxBufN instead of Box<BufN>.  Use BoxBuf256 instead of Box<Buf256>.
            ///
            /// Standard heaps allocate memory in blocks.  The block sizes are powers of two and
            /// some intervening sizes.  For example, jemalloc's
            /// [block sizes](http://jemalloc.net/jemalloc.3.html#size_classes) are
            /// 8, 16, 32, 48, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384, 448, 512,
            /// and so on.  Thus jemalloc uses 320 bytes to store a 257 byte value.
            ///
            /// Every Buf contains two `usize` index values in addition to its buffer memory.
            /// Since BufN uses powers of two buffer sizes, the Buf struct size is always a few
            /// bytes larger than a power of two, and takes up the next larger block size on the
            /// heap.  For example, in a 64-bit binary using jemalloc,
            /// Box<Buf256> uses 256 + 8 + 8 (buffer + read_index + write_index) = 272 bytes
            /// stored in a 320 byte block, wasting 15% of memory.
            ///
            /// BoxBuf256 works around this by allocating its buffer memory separately,
            /// as a Box<[u8; 256]> which uses a 256-byte block and wastes no memory.
            ///
            /// See the program examples/box_buf_mem_benchmark for a real benchmark showing the
            /// efficiency differences.
            pub struct $NAME {
                mem: Box<[u8; $LEN]>,
                field1: bool,
            }
            impl $NAME {
                pub fn new() -> $NAME {
                    $NAME {
                        mem: Box::new(['.' as u8; $LEN]),
                        field1: false,
                    }
                }
            }
            impl Buf for $NAME {
                fn mem_mut(&mut self) -> &mut [u8] {
                    &mut self.mem[..]
                }

                fn mem(&self) -> &[u8] {
                    &self.mem[..]
                }

                fn field1(&self) -> bool {
                    self.field1
                }

                fn set_field1(&mut self, v: bool) {
                    self.field1 = v;
                }
            }
            impl std::fmt::Debug for $NAME {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    self.debug(stringify!($NAME), f)
                }
            }
        )+
    }
}

generate_box_buf_n_structs! {
    BoxBuf256, 256
    BoxBuf512, 512
    BoxBuf1KB, 1024
    BoxBuf2KB, 2 * 1024
    BoxBuf4KB, 4 * 1024
    BoxBuf8KB, 8 * 1024
    BoxBuf16KB, 16 * 1024
    BoxBuf32KB, 32 * 1024
    BoxBuf64KB, 64 * 1024
    BoxBuf128KB, 128 * 1024
    BoxBuf256KB, 256 * 1024
    BoxBuf512KB, 512 * 1024
    BoxBuf1MB, 1024 * 1024
    BoxBuf2MB, 2 * 1024 * 1024
    BoxBuf4MB, 4 * 1024 * 1024
    BoxBuf8MB, 8 * 1024 * 1024
    BoxBuf16MB, 16 * 1024 * 1024
    BoxBuf32MB, 32 * 1024 * 1024
    BoxBuf64MB, 64 * 1024 * 1024
    BoxBuf128MB, 128 * 1024 * 1024
    BoxBuf256MB, 256 * 1024 * 1024
}

use clap::arg_enum;

arg_enum! {
    #[derive(Debug)]
    enum BufType {
        Buf,
        BoxBuf
    }
}

#[derive(structopt::StructOpt, Debug)]
struct Opt {
    #[structopt(possible_values = &BufType::variants(), case_insensitive = true, default_value = "Buf")]
    buf_type: BufType,
}

fn print_active_mem() {
    jemalloc_ctl::epoch::mib().unwrap().advance().unwrap();
    println!(
        "{:.1} MiB active memory",
        (jemalloc_ctl::stats::active::read().unwrap() as f64) / (1024.0 * 1024.0)
    );
}

use jemallocator;
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() {
    let opt: Opt = structopt::StructOpt::from_args();
    match opt.buf_type {
        BufType::Buf => {
            println!("Making 1M Buf256 structs");
            let mut v: Vec<Buf256> = Vec::new();
            for _ in 0..(1024 * 1024) {
                v.push(Buf256::new());
            }
            print_active_mem();
            //v[0].modify_and_get_buf();
        }
        BufType::BoxBuf => {
            println!("Making 1M BoxBuf256 structs");
            let mut v: Vec<BoxBuf256> = Vec::new();
            for _ in 0..(1024 * 1024) {
                v.push(BoxBuf256::new());
            }
            print_active_mem();
            //v[0].modify_and_get_buf();
        }
    }
}

// $ cargo run --package fixed-buffer --example server
// SERVER listening on 127.0.0.1:61779
// CLIENT connecting
// CLIENT sending requests
// SERVER handling connection
// CLIENT got response "ad98e545"
// CLIENT got response "HI"
