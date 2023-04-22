use accept_encoding::{match_for_encoding, match_for_encoding_monolith_for_benchmark};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_fibs(c: &mut Criterion) {
    let mut group = c.benchmark_group("match_for_encoding");
    let input_values: Vec<&[u8]> = vec![b"gzip, deflate, br", b"gzip, deflate"];
    let encoding = b"br";
    for i in 0..input_values.len() {
        group.bench_with_input(BenchmarkId::new("modular_parser", i), &i, |b, i| {
            b.iter(|| black_box(match_for_encoding(input_values[*i], encoding)))
        });
        group.bench_with_input(BenchmarkId::new("monolith_parser", i), &i, |b, i| {
            b.iter(|| {
                black_box(match_for_encoding_monolith_for_benchmark(
                    input_values[*i],
                    encoding,
                ))
            })
        });
    }
}

criterion_group!(benches, bench_fibs);
criterion_main!(benches);
