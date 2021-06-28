use std::time::Instant;
use rayon::prelude::*;
use modular::*;
use fast_tracer::stats;
use tracing::{span, Level};

const K: u32 = 7907;
const M: i32 = 256;

fn timed<T>(body: impl FnOnce() -> T) -> (T, std::time::Duration) { let start = Instant::now();
    let result = body();
    let time_taken = start.elapsed();
    (result, time_taken)
}

fn mod_exp(b: i32, e: i32, k: u32) -> Modulo {
    let mut mul = 1.to_modulo(k);
    let b = b.to_modulo(k);
    for _ in 0..e {
        mul = mul * b;
    }
    
    mul
}

fn string_match(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let (_, needle_hash) = needle.iter().rev().fold((1.to_modulo(K), 0.to_modulo(K)), |(mut mul, mut sum), &x| {
        let x = (x as i32).to_modulo(K);
      
        sum = sum + (x * mul);
        mul = mul * M.to_modulo(K);

        (mul, sum)
    });

    let needle_hash = needle_hash.remainder();

    let block_size = haystack.len();

    haystack
        .par_windows(needle.len())
        .enumerate()
        .adaptive(block_size)
        .scan(|| None, |state: &mut Option<(u8, i32)>, (index, win)| {
            *state = state
                .map(|(first, prev_hash)| {
                    let first = first as i32;
                    let e = win.len() as i32 - 1;
                    mod_exp(prev_hash - first, e, K);
                    let new_hash = (prev_hash.to_modulo(K) - first.to_modulo(K) * mod_exp(M, e, K)) * M.to_modulo(K)
                        + (*win.last().unwrap() as i32).to_modulo(K);
                    (win[0], new_hash.remainder())
                })
                .or_else(|| {
                    let span = span!(Level::TRACE, "init");
                    let _guard = span.enter();
                    let hash = win
                        .iter()
                        .rev()
                        .fold((1.to_modulo(K), 0.to_modulo(K)), |(mut mul, mut sum), &x| {
                            let x = (x as i32).to_modulo(K);
                            
                            sum = sum + x * mul;
                            mul = mul * M.to_modulo(K);

                            (mul, sum)
                        })
                        .1;

                    Some((win[0], hash.remainder()))
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
