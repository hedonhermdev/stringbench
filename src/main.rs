#![feature(test)]
use bare_metal_modulo::*;
use fast_tracer::{stats, svg};
use lipsum::lipsum_words_from_seed;
use num::traits::Pow;
use rayon::prelude::*;
use tracing::{Level,span};
use std::time::{Instant, Duration};

const K: u32 = 7907;
const M: u32 = 256;

fn timed<T>(body: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let result = body();
    let time_taken = start.elapsed();
    (result, time_taken)
}

fn string_match(haystack: &[u8], needle: &[u8], target_time: Duration) -> usize {
    let weight = ModNum::new(M, K).pow(needle.len() as u32 - 1);
    let needle_hash = needle.iter().fold(ModNum::new(0, K), |old, &x| {
        old * M + ModNum::new(x as u32, K)
    });

    let windows = haystack.par_windows(needle.len()).enumerate();

    let span = span!(Level::TRACE, "iter_fold");
    let _guard = span.enter();
    windows
        .adaptive(target_time)
        .scan(
            || None,
            |state, (index, win)| {
                *state = state
                    .map(|(first, prev_hash)| {
                        let first = ModNum::new(first as u32, K);
                        let last = ModNum::new(*win.last().unwrap() as u32, K);
                        let new_hash = (prev_hash - first * weight) * M + last;

                        (win[0], new_hash)
                    })
                    .or_else(|| {
                        let hash = win.iter().fold(ModNum::new(0, K), |old, &x| {
                            old * M + ModNum::new(x as u32, K)
                        });

                        Some((win[0], hash))
                    });

                state.map(|(_, h)| (index, h))
            },
        )
        .filter(|(_, h)| *h == needle_hash)
        .map(|(index, _)| index)
        .count()
}

fn main() {
    let haystack = lipsum_words_from_seed(600_000, 0);
    let needle = haystack.chars().take(300_000).collect::<String>();
    eprintln!("built haystack {}", haystack.len());

    println!("scheme,num_threads,time_taken");

    stats(|| {
        let (_count, time_taken) =
        timed(|| string_match(haystack.as_bytes(), needle.as_bytes(), Duration::from_nanos(400000)));
        println!("{},{},{}", "time_based", 4, time_taken.as_nanos());
    });
}
