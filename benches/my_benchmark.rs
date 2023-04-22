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

#[rustfmt::skip]
const TCHAR_TABLE: [bool; 256] = [
    // tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
    //         "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, true,  false, true,  true,  true,  true,  true,  false, false, true,  true,  false, true,  true,  false,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  false, false, false, false, false, false,
    false, true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  false, false, false, true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,
    true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  true,  false, true,  false, true,  false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
    false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false,
];

#[inline]
fn is_tchar_with_table(c: u8) -> bool {
    TCHAR_TABLE[c as usize]
}

#[inline]
fn is_tchar_with_match(c: u8) -> bool {
    match c {
        // token = 1*tchar
        // tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
        //         "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
        b'!'
        | b'#'
        | b'$'
        | b'%'
        | b'&'
        | b'\''
        | b'*'
        | b'+'
        | b'-'
        | b'.'
        | b'^'
        | b'_'
        | b'`'
        | b'|'
        | b'~'
        | b'0'..=b'9'
        | b'A'..=b'Z'
        | b'a'..=b'z' => true,
        _ => false,
    }
}

fn bench_is_tchar(c: &mut Criterion) {
    let mut group = c.benchmark_group("is_tchar");
    let input_values: Vec<&[u8]> = vec![b"gzip, deflate, br", b"gzip, deflate"];
    for i in 0..input_values.len() {
        group.bench_with_input(BenchmarkId::new("table", i), &i, |b, i| {
            b.iter(|| {
                for c in input_values[*i].iter() {
                    black_box(is_tchar_with_table(*c));
                }
            })
        });
        group.bench_with_input(BenchmarkId::new("match", i), &i, |b, i| {
            b.iter(|| {
                for c in input_values[*i].iter() {
                    black_box(is_tchar_with_match(*c));
                }
            })
        });
    }
}
criterion_group!(benches, bench_is_tchar);
criterion_main!(benches);
