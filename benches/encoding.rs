use accept_encoding::{encoding_matcher2, match_for_encoding};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("encoding");
    let input_values: Vec<&[u8]> = vec![b"gzip, deflate, br", b"gzip, deflate"];
    let encoding = b"br";
    for i in 0..input_values.len() {
        group.bench_with_input(BenchmarkId::new("lexer_combinator", i), &i, |b, i| {
            b.iter(|| {
                black_box(encoding_matcher2::match_for_encoding(
                    input_values[*i],
                    encoding,
                ))
            })
        });
    }
    for i in 0..input_values.len() {
        group.bench_with_input(BenchmarkId::new("modular_parser", i), &i, |b, i| {
            b.iter(|| black_box(match_for_encoding(input_values[*i], encoding)))
        });
    }
}

criterion_group!(benches, encoding);
criterion_main!(benches);