//! Performance benchmarks for TOON encoding and decoding.
//!
//! Run with: cargo bench
//!
//! Benchmarks cover:
//! - Encode small/medium/large JSON
//! - Decode small/medium/large TOON
//! - Key folding overhead
//! - Tabular array detection
//! - Comparison against `serde_json` baseline

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use toon_rust::options::{EncodeOptions, KeyFoldingMode};
use toon_rust::{decode, encode};

// ============================================================================
// TEST DATA GENERATION
// ============================================================================

/// Generate a simple object with the given number of fields
fn generate_simple_object(num_fields: usize) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for i in 0..num_fields {
        obj.insert(
            format!("field_{i}"),
            serde_json::json!(format!("value_{i}")),
        );
    }
    serde_json::Value::Object(obj)
}

/// Generate a nested object with the given depth
fn generate_nested_object(depth: usize) -> serde_json::Value {
    let mut value = serde_json::json!({"leaf": "value"});
    for i in 0..depth {
        value = serde_json::json!({ format!("level_{}", i): value });
    }
    value
}

/// Generate a tabular array (array of objects with same structure)
#[allow(clippy::cast_precision_loss)]
fn generate_tabular_array(num_rows: usize) -> serde_json::Value {
    let rows: Vec<serde_json::Value> = (0..num_rows)
        .map(|i| {
            serde_json::json!({
                "id": i as f64,
                "name": format!("User {}", i),
                "email": format!("user{}@example.com", i),
                "active": i % 2 == 0
            })
        })
        .collect();
    serde_json::Value::Array(rows)
}

/// Generate a deeply nested structure for key folding
fn generate_foldable_structure(chains: usize, depth: usize) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for chain in 0..chains {
        let mut value = serde_json::json!(format!("value_{}", chain));
        for level in 0..depth {
            value = serde_json::json!({ format!("l{}", level): value });
        }
        obj.insert(format!("chain_{chain}"), value);
    }
    serde_json::Value::Object(obj)
}

// ============================================================================
// ENCODE BENCHMARKS
// ============================================================================

fn bench_encode_small(c: &mut Criterion) {
    let json = generate_simple_object(5);
    let json_str = serde_json::to_string(&json).unwrap();

    let mut group = c.benchmark_group("encode_small");
    group.throughput(Throughput::Bytes(json_str.len() as u64));

    group.bench_function("toon_encode", |b| {
        b.iter(|| encode(black_box(json.clone()), None));
    });

    group.bench_function("serde_json_to_string", |b| {
        b.iter(|| serde_json::to_string(black_box(&json)));
    });

    group.bench_function("serde_json_to_string_pretty", |b| {
        b.iter(|| serde_json::to_string_pretty(black_box(&json)));
    });

    group.finish();
}

fn bench_encode_medium(c: &mut Criterion) {
    let json = generate_simple_object(100);
    let json_str = serde_json::to_string(&json).unwrap();

    let mut group = c.benchmark_group("encode_medium");
    group.throughput(Throughput::Bytes(json_str.len() as u64));

    group.bench_function("toon_encode", |b| {
        b.iter(|| encode(black_box(json.clone()), None));
    });

    group.bench_function("serde_json_to_string", |b| {
        b.iter(|| serde_json::to_string(black_box(&json)));
    });

    group.finish();
}

fn bench_encode_large(c: &mut Criterion) {
    let json = generate_simple_object(1000);
    let json_str = serde_json::to_string(&json).unwrap();

    let mut group = c.benchmark_group("encode_large");
    group.throughput(Throughput::Bytes(json_str.len() as u64));

    group.bench_function("toon_encode", |b| {
        b.iter(|| encode(black_box(json.clone()), None));
    });

    group.bench_function("serde_json_to_string", |b| {
        b.iter(|| serde_json::to_string(black_box(&json)));
    });

    group.finish();
}

fn bench_encode_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_nested");

    for depth in [10, 25, 50, 100] {
        let json = generate_nested_object(depth);
        let json_str = serde_json::to_string(&json).unwrap();
        group.throughput(Throughput::Bytes(json_str.len() as u64));

        group.bench_with_input(BenchmarkId::new("toon", depth), &json, |b, json| {
            b.iter(|| encode(black_box(json.clone()), None));
        });
    }

    group.finish();
}

fn bench_encode_tabular(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_tabular");

    for rows in [10, 100, 1000] {
        let json = generate_tabular_array(rows);
        let json_str = serde_json::to_string(&json).unwrap();
        group.throughput(Throughput::Bytes(json_str.len() as u64));

        group.bench_with_input(BenchmarkId::new("toon", rows), &json, |b, json| {
            b.iter(|| encode(black_box(json.clone()), None));
        });

        group.bench_with_input(BenchmarkId::new("serde_json", rows), &json, |b, json| {
            b.iter(|| serde_json::to_string(black_box(json)));
        });
    }

    group.finish();
}

// ============================================================================
// DECODE BENCHMARKS
// ============================================================================

fn bench_decode_small(c: &mut Criterion) {
    let json = generate_simple_object(5);
    let toon = encode(json.clone(), None);
    let json_str = serde_json::to_string(&json).unwrap();

    let mut group = c.benchmark_group("decode_small");
    group.throughput(Throughput::Bytes(toon.len() as u64));

    group.bench_function("toon_decode", |b| {
        b.iter(|| decode(black_box(&toon), None));
    });

    group.bench_function("serde_json_from_str", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(&json_str)));
    });

    group.finish();
}

fn bench_decode_medium(c: &mut Criterion) {
    let json = generate_simple_object(100);
    let toon = encode(json.clone(), None);
    let json_str = serde_json::to_string(&json).unwrap();

    let mut group = c.benchmark_group("decode_medium");
    group.throughput(Throughput::Bytes(toon.len() as u64));

    group.bench_function("toon_decode", |b| {
        b.iter(|| decode(black_box(&toon), None));
    });

    group.bench_function("serde_json_from_str", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(&json_str)));
    });

    group.finish();
}

fn bench_decode_large(c: &mut Criterion) {
    let json = generate_simple_object(1000);
    let toon = encode(json.clone(), None);
    let json_str = serde_json::to_string(&json).unwrap();

    let mut group = c.benchmark_group("decode_large");
    group.throughput(Throughput::Bytes(toon.len() as u64));

    group.bench_function("toon_decode", |b| {
        b.iter(|| decode(black_box(&toon), None));
    });

    group.bench_function("serde_json_from_str", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(&json_str)));
    });

    group.finish();
}

fn bench_decode_tabular(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_tabular");

    for rows in [10, 100, 1000] {
        let json = generate_tabular_array(rows);
        let toon = encode(json.clone(), None);
        group.throughput(Throughput::Bytes(toon.len() as u64));

        group.bench_with_input(BenchmarkId::new("toon", rows), &toon, |b, toon| {
            b.iter(|| decode(black_box(toon), None));
        });
    }

    group.finish();
}

// ============================================================================
// KEY FOLDING BENCHMARKS
// ============================================================================

fn bench_key_folding_overhead(c: &mut Criterion) {
    let json = generate_foldable_structure(10, 5);

    let mut group = c.benchmark_group("key_folding");

    group.bench_function("without_folding", |b| {
        let options = Some(EncodeOptions {
            indent: None,
            delimiter: None,
            key_folding: Some(KeyFoldingMode::Off),
            flatten_depth: None,
            replacer: None,
        });
        b.iter(|| encode(black_box(json.clone()), options.clone()));
    });

    group.bench_function("with_folding", |b| {
        let options = Some(EncodeOptions {
            indent: None,
            delimiter: None,
            key_folding: Some(KeyFoldingMode::Safe),
            flatten_depth: None,
            replacer: None,
        });
        b.iter(|| encode(black_box(json.clone()), options.clone()));
    });

    group.finish();
}

// ============================================================================
// COMPRESSION RATIO BENCHMARKS
// ============================================================================

#[allow(clippy::cast_precision_loss)]
fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_ratio");

    // Tabular data (best case for TOON)
    let tabular = generate_tabular_array(100);
    let tabular_json = serde_json::to_string(&tabular).unwrap();
    let tabular_toon = encode(tabular.clone(), None);

    println!(
        "Tabular: JSON {} bytes, TOON {} bytes, ratio {:.1}%",
        tabular_json.len(),
        tabular_toon.len(),
        (tabular_toon.len() as f64 / tabular_json.len() as f64) * 100.0
    );

    // Simple object (moderate case)
    let simple = generate_simple_object(50);
    let simple_json = serde_json::to_string(&simple).unwrap();
    let simple_toon = encode(simple, None);

    println!(
        "Simple: JSON {} bytes, TOON {} bytes, ratio {:.1}%",
        simple_json.len(),
        simple_toon.len(),
        (simple_toon.len() as f64 / simple_json.len() as f64) * 100.0
    );

    // Nested (with folding)
    let nested = generate_foldable_structure(5, 10);
    let nested_json = serde_json::to_string(&nested).unwrap();
    let nested_toon_unfolded = encode(nested.clone(), None);
    let options_folded = Some(EncodeOptions {
        indent: None,
        delimiter: None,
        key_folding: Some(KeyFoldingMode::Safe),
        flatten_depth: None,
        replacer: None,
    });
    let nested_toon_folded = encode(nested, options_folded);

    println!(
        "Nested: JSON {} bytes, TOON (unfolded) {} bytes, TOON (folded) {} bytes",
        nested_json.len(),
        nested_toon_unfolded.len(),
        nested_toon_folded.len()
    );

    // Just run a simple benchmark to include in the report
    group.bench_function("tabular_encode", |b| {
        b.iter(|| encode(black_box(tabular.clone()), None));
    });

    group.finish();
}

// ============================================================================
// ROUNDTRIP BENCHMARKS
// ============================================================================

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    for size in [10, 100, 500] {
        let json = generate_simple_object(size);
        let json_str = serde_json::to_string(&json).unwrap();

        group.bench_with_input(
            BenchmarkId::new("toon_roundtrip", size),
            &json,
            |b, json| {
                b.iter(|| {
                    let toon = encode(black_box(json.clone()), None);
                    decode(black_box(&toon), None)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("serde_json_roundtrip", size),
            &json_str,
            |b, json_str| {
                b.iter(|| {
                    let parsed: serde_json::Value =
                        serde_json::from_str(black_box(json_str)).unwrap();
                    serde_json::to_string(black_box(&parsed)).unwrap()
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// CRITERION GROUPS
// ============================================================================

criterion_group!(
    benches,
    bench_encode_small,
    bench_encode_medium,
    bench_encode_large,
    bench_encode_nested,
    bench_encode_tabular,
    bench_decode_small,
    bench_decode_medium,
    bench_decode_large,
    bench_decode_tabular,
    bench_key_folding_overhead,
    bench_compression_ratio,
    bench_roundtrip,
);

criterion_main!(benches);
