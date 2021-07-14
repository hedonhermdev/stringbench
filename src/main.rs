#![feature(test)]
use bare_metal_modulo::*;
use fast_tracer::{stats, svg};
use lipsum::lipsum_words_from_seed;
use num::traits::Pow;
use rayon::prelude::*;
use std::time::Instant;

use tracing::{span, Level};

extern crate test;

const K: u32 = 7907;
const M: u32 = 256;

fn timed<T>(body: impl FnOnce() -> T) -> (T, std::time::Duration) {
    let start = Instant::now();
    let result = body();
    let time_taken = start.elapsed();
    (result, time_taken)
}

fn string_match(haystack: &[u8], needle: &[u8], block_size: usize) -> usize {
    let weight = ModNum::new(M, K).pow(needle.len() as u32 - 1);
    let needle_hash = needle.iter().fold(ModNum::new(0, K), |old, &x| {
        old * M + ModNum::new(x as u32, K)
    });

    let windows = haystack.par_windows(needle.len()).enumerate();

    windows
        .adaptive(block_size)
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
    let haystack = lipsum_words_from_seed(60_000_000, 0);
    println!("built haystack {}", haystack.len());

    let needle = haystack.chars().take(10_000_000).collect::<String>();

    let mut block_size = 100;

    while block_size < haystack.len() {
        stats(|| {
            let (_count, time_taken) =
                timed(|| string_match(haystack.as_bytes(), needle.as_bytes(), block_size));
                println!("{}, {}", block_size, time_taken.as_nanos());
        });

        block_size *= 2;
    }

    // for n in 1..=64 {
    //     let tp = rayon::ThreadPoolBuilder::new().num_threads(n).build().unwrap();

    //     let mut sum = 0;
    //     for _ in 0..30 {
    //         let (count, time_taken) = tp.install(|| stats(|| timed(|| string_match(haystack.as_bytes(), needle.as_bytes(), true))));
    //         sum += time_taken.as_nanos();
    //     }
    //     let avg = sum / 30;
    //     println!("{},{},{}", &tp.current_num_threads(), "adaptive", avg);
    // }

    //     for size in (1000..100_000).step_by(100) {
    //         let needle: String = haystack.chars().take(size).collect();
    //         let (count1, time_taken) = timed(|| string_match(haystack.as_bytes(), needle.as_bytes(), false));
    //         println!("{},{},{}", &size, "rayon", time_taken.as_nanos());
    //         let (count2, time_taken) = timed(|| string_match(haystack.as_bytes(), needle.as_bytes(), true));
    //         println!("{},{},{}", &size, "adaptive", time_taken.as_nanos());
    //         assert_eq!(count1, count2);
    //     }
}

#[cfg(test)]
mod tests {
    use super::string_match;

    use lipsum::lipsum_words_from_seed;
    use test::Bencher;

    #[bench]
    fn bench_adaptive(b: &mut Bencher) {
        b.iter(|| {
            let haystack = lipsum_words_from_seed(100_000, 0);
            let needle = haystack.chars().take(10_000).collect::<String>();

            let matches = string_match(haystack.as_bytes(), needle.as_bytes(), 10000);

            matches
        })
    }

    #[test]
    fn match_exists() {
        let haystack = "applepineapplepenpineapplepen";
        let needle = "apple";

        let matches = string_match(haystack.as_bytes(), needle.as_bytes(), true);
    }
}
