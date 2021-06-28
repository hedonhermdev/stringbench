use bare_metal_modulo::*;
use fast_tracer::{stats, svg};
use lipsum::lipsum_words_from_seed;
use num::{traits::Pow, ToPrimitive};
use rayon::prelude::*;
use std::time::Instant;
use tracing::{span, Level};

const K: u32 = 7907;
const M: u32 = 256;

fn timed<T>(body: impl FnOnce() -> T) -> (T, std::time::Duration) {
    let start = Instant::now();
    let result = body();
    let time_taken = start.elapsed();
    (result, time_taken)
}

fn string_match(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let weight = ModNum::new(M, K).pow(needle.len() as u32 - 1);
    let needle_hash = needle.iter().fold(ModNum::new(0, K), |old, &x| {
        old * M + ModNum::new(x as u32, K)
    });

    let block_size = haystack.len();

    let hashes = haystack
        .par_windows(needle.len())
        .enumerate()
        // .adaptive(block_size)
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
                        let span = span!(Level::TRACE, "init");
                        let _guard = span.enter();
                        let hash = win.iter().fold(ModNum::new(0, K), |old, &x| {
                            old * M + ModNum::new(x as u32, K)
                        });

                        Some((win[0], hash))
                    });

                state.map(|(_, h)| (index, h))
            }).collect::<Vec<_>>();

    let correct_hashes = haystack
        .windows(needle.len())
        .enumerate()
        .scan(None, 
            |state, (index, win)| {
                *state = state
                    .map(|(first, prev_hash)| {
                        let first = ModNum::new(first as u32, K);
                        let last = ModNum::new(*win.last().unwrap() as u32, K);
                        let new_hash = (prev_hash - first * weight) * M + last;

                        (win[0], new_hash)
                    })
                    .or_else(|| {
                        let span = span!(Level::TRACE, "init");
                        let _guard = span.enter();
                        let hash = win.iter().fold(ModNum::new(0, K), |old, &x| {
                            old * M + ModNum::new(x as u32, K)
                        });

                        Some((win[0], hash))
                    });

                state.map(|(_, h)| (index, h))
    }).collect::<Vec<_>>();

    assert_eq!(correct_hashes, hashes);

    hashes
        .iter()
        .filter(|(_, h)| *h == needle_hash)
        .map(|(index, _)| *index)
        .collect()
}

fn main() {
    // let haystack = include_bytes!("../text.txt");

    // let needle = "penpineapplepenpineapplepenapple".as_bytes();

    let haystack = lipsum_words_from_seed(100_000, 0);
    let needle = haystack.chars().take(10_000).collect::<String>();
    stats(|| {
        let (matches, time_taken) = timed(|| string_match(haystack.as_bytes(), needle.as_bytes()));
        println!("time: {}", time_taken.as_nanos());
        println!("{:?}", matches);
        
        // assert_eq!(matches.is_empty(), false);
    });
}

#[cfg(test)]
mod tests {
    use super::string_match;

    #[test]
    fn match_exists() {
        let haystack = "applepineapplepenpineapplepen";
        let needle = "apple";

        let mut matches = string_match(haystack.as_bytes(), needle.as_bytes());
        matches.sort();

        assert_eq!(matches, [0, 9, 21]);
    }
}
