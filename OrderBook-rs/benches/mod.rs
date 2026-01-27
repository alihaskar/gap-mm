use criterion::{criterion_group, criterion_main};

mod concurrent;
mod order_book;
mod simple;

use concurrent::register_benchmarks as register_concurrent_benchmarks;
use order_book::register_benchmarks as register_order_book_benchmarks;
use simple::basic::benchmark_data;

// Define the benchmark groups
criterion_group!(
    benches,
    benchmark_data,
    register_order_book_benchmarks,
    register_concurrent_benchmarks,
);

criterion_main!(benches);
