use std::time::Instant;
use bare_metal_modulo::*;
use rayon::prelude::*;
use fast_tracer::stats;
use tracing::{span, Level};
use num::{ToPrimitive, traits::Pow};

const K: u32 = 7907;
const M: u32 = 256;

fn timed<T>(body: impl FnOnce() -> T) -> (T, std::time::Duration) { let start = Instant::now();
    let result = body();
    let time_taken = start.elapsed();
    (result, time_taken)
}

fn string_match(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let weight = ModNum::new(M, K).pow(ModNum::new(needle.len().to_u32().unwrap() - 1, K));
    let needle_hash = needle.iter().rev().fold(ModNum::new(0, K), |old , &x| old*M + ModNum::new(x as u32, K));

    let block_size = haystack.len();

    haystack
        .par_windows(needle.len())
        .enumerate()
        .adaptive(block_size)
        .scan(|| None, |state, (index, win)| {
            *state = state.map(|(first, prev_hash)| {
                let first = ModNum::new(first as u32, K);
                let last = ModNum::new(*win.last().unwrap() as u32, K);
                let new_hash = (prev_hash - first * weight) * M + last;

                (win[0], new_hash)
            })
            .or_else(|| {
                let span = span!(Level::TRACE, "init");
                let _guard = span.enter();
                let hash = win.iter().rev().fold(ModNum::new(0, K), |old , &x| old * M + ModNum::new(x as u32, K));

                Some((win[0], hash))
            });

            state.map(|(_, h)| (index, h))
        })
    .filter(|(_, h)| *h == needle_hash)
        .map(|(index, _)| index)
        .collect()
}

fn main() {
    let haystack = include_bytes!("../text.txt");

    let needle = "penpineapplepenpineapplepenapple".as_bytes();

    let (matches, time_taken) = stats(|| timed(|| string_match(haystack, needle)));

    println!("time: {}", time_taken.as_nanos());
    println!("matches: {:?}", matches);
}

#[cfg(test)]
mod tests {
    use super::string_match;

    #[test]
    fn match_exists() {
        let haystack = "applepineapplepenpineapplepen";
        let needle = "apple";

        let matches = string_match(haystack.as_bytes(), needle.as_bytes());

        assert_eq!(matches, [0, 9, 21]);
    }
}
