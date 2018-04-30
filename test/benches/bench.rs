#![feature(test)]

extern crate test;
extern crate wee_alloc;
extern crate wee_alloc_test;

use std::io;
use wee_alloc_test::*;

macro_rules! bench_trace {
    ($name:ident, $trace:expr) => {
        #[bench]
        #[cfg(not(feature = "extra_assertions"))]
        fn $name(b: &mut test::Bencher) {
            let operations = Operations::read_trace($trace);

            {
                let stdout = io::stdout();
                let _stdout = stdout.lock();

                println!(
                    "################## {} ##################",
                    stringify!($name)
                );
                println!("#");
                println!("# Allocations by log2(Size)");
                println!("#");
                println!("{}", operations.size_histogram());
                println!("#");
                println!("# Allocations by Lifetime");
                println!("#");
                println!("{}", operations.lifetime_histogram());
            }

            let a = &wee_alloc::WeeAlloc::INIT;
            b.iter(|| {
                operations.run_with_allocator(a);
            });
        }
    };
}

bench_trace!(bench_trace_cpp_demangle, "../traces/cpp-demangle.trace");
bench_trace!(bench_trace_dogfood, "../traces/dogfood.trace");
bench_trace!(bench_trace_ffmpeg, "../traces/ffmpeg.trace");
bench_trace!(bench_trace_find, "../traces/find.trace");
bench_trace!(bench_trace_gcc_hello, "../traces/gcc-hello.trace");
bench_trace!(
    bench_trace_grep_random_data,
    "../traces/grep-random-data.trace"
);
bench_trace!(bench_trace_grep_recursive, "../traces/grep-recursive.trace");
bench_trace!(bench_trace_ls, "../traces/ls.trace");
bench_trace!(bench_trace_source_map, "../traces/source-map.trace");
