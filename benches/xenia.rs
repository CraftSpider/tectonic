//! Benchmarks for the Tectonic engine

#![allow(missing_docs)]

use criterion::Criterion;
use std::path::Path;
use std::{env, fs};
use tectonic::latex_to_pdf;

fn criterion() -> Criterion {
    let c = Criterion::default().configure_from_args().sample_size(10);
    #[cfg(unix)]
    let c = c.with_profiler(pprof::criterion::PProfProfiler::new(
        5000,
        pprof::criterion::Output::Flamegraph(None),
    ));
    c
}

fn xenia(c: &mut Criterion) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/xenia");
    env::set_current_dir(&dir).unwrap();
    let data = fs::read_to_string(dir.join("paper.tex")).unwrap();
    c.bench_function("xenia", |b| {
        b.iter(|| {
            latex_to_pdf(&data).unwrap();
        })
    });
}

criterion::criterion_group!(name = outputs; config = criterion(); targets = xenia);
criterion::criterion_main!(outputs);
