use accept_encoding::match_for_encoding;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn bench_fibs(c: &mut Criterion) {
    let mut group = c.benchmark_group("match_for_encoding");
    let input_values: Vec<&[u8]> = vec![b"gzip, deflate, br", b"gzip, deflate"];
    let encoding = b"br";
    for i in 0..input_values.len() {
        group.bench_with_input(BenchmarkId::new("modular_parser", i), &i, |b, i| {
            b.iter(|| black_box(match_for_encoding(input_values[*i], encoding)))
        });
    }
}

#[inline]
fn is_ascii_lowercase_stdlib(c: u8) -> bool {
    c.is_ascii_lowercase()
}

#[inline]
fn is_ascii_lowercase_contains(c: u8) -> bool {
    (b'a'..=b'z').contains(&c)
}

#[inline]
fn is_ascii_lowercase_primitive(c: u8) -> bool {
    b'a' <= c && c <= b'z'
}

fn bench_is_ascii_lowercase(c: &mut Criterion) {
    let mut group = c.benchmark_group("is_ascii_lowercase");
    group.bench_function("stdlib", |b| {
        b.iter(|| {
            for c in b'\x00'..=b'\xFF' {
                black_box(is_ascii_lowercase_stdlib(c));
            }
        })
    });
    group.bench_function("contains", |b| {
        b.iter(|| {
            for c in b'\x00'..=b'\xFF' {
                black_box(is_ascii_lowercase_contains(c));
            }
        })
    });
    group.bench_function("primitive", |b| {
        b.iter(|| {
            for c in b'\x00'..=b'\xFF' {
                black_box(is_ascii_lowercase_primitive(c));
            }
        })
    });
}

criterion_group!(benches, bench_is_ascii_lowercase);
criterion_main!(benches);
