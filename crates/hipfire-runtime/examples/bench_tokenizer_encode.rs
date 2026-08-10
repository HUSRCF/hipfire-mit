// SPDX-License-Identifier: MIT
//! Reproducible tokenizer encode microbenchmark.
//!
//! Usage: bench_tokenizer_encode <model.hfq> [iterations]

use hipfire_runtime::hfq::HfqFile;
use hipfire_runtime::tokenizer::Tokenizer;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

fn measure(tokenizer: &Tokenizer, label: &str, text: &str, iterations: usize) {
    for _ in 0..32 {
        black_box(tokenizer.encode(black_box(text)));
    }

    let mut samples = Vec::with_capacity(7);
    let mut token_count = 0usize;
    for _ in 0..7 {
        let start = Instant::now();
        for _ in 0..iterations {
            let ids = tokenizer.encode(black_box(text));
            token_count = ids.len();
            black_box(ids);
        }
        samples.push(start.elapsed().as_nanos() as f64 / iterations as f64);
    }
    samples.sort_by(f64::total_cmp);
    let median_ns = samples[samples.len() / 2];
    let mib_s = text.len() as f64 / median_ns * 1_000_000_000.0 / (1024.0 * 1024.0);
    println!(
        "{label}: bytes={} tokens={} iterations={} median_ns={median_ns:.1} MiB_s={mib_s:.3}",
        text.len(), token_count, iterations
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model = args.get(1).expect("usage: bench_tokenizer_encode <model.hfq> [iterations]");
    let iterations = args.get(2).and_then(|value| value.parse().ok()).unwrap_or(2_000);

    let hfq = HfqFile::open(Path::new(model)).expect("open HFQ");
    let tokenizer = Tokenizer::from_hfq_metadata(&hfq.metadata_json).expect("load tokenizer");
    let plain = "Explain why stable interfaces make systems easier to evolve without changing observable behavior.";
    let framed = "<|im_start|>user\nExplain why stable interfaces matter.<|im_end|>\n<|im_start|>assistant\n";

    measure(&tokenizer, "plain", plain, iterations);
    measure(&tokenizer, "framed", framed, iterations);
}
