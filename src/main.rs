use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs;
use std::ops::{Deref, DerefMut};
use std::time::Instant;

use rayon::prelude::{IntoParallelIterator, ParallelIterator};

fn main() {
    let words = fs::read_to_string("words.txt").unwrap();

    let timer = Instant::now();

    process(&words);

    println!("Elapsed ms [{}]", timer.elapsed().as_millis());
}

fn process(all_words: &String) {
    let mut seen: HashSet<u32> = Default::default();
    let mut words: Vec<_> = Vec::with_capacity(6000);
    let mut freq: [(ZChar, u32); 26] = Default::default();

    for i in 0..26 {
        freq[i].0 = ZChar(i as u8)
    }

    'index_words: for word in all_words.lines() {
        // we are looking for 5-letter words ONLY!
        //
        if word.len() != 5 {
            continue;
        }

        let mut bits = 0;
        let mut zwrd: ZWord = Default::default();

        for (i, c) in word.chars().enumerate() {
            let z = ZChar::from(c);
            let b = z.mask();

            // if we get a duplicate letter (e.g. floor - has two o's)
            // this isn't a valid 5-letter word as all letters MUST
            // appear only ONCE
            //
            if bits & b != 0 {
                continue 'index_words;
            }

            // add this letter to the word bitfield, and increase
            // the letter frequency count
            //
            bits |= b;
            zwrd[i] = z;
            freq[z.ord()].1 += 1;
        }

        // we don't need anagrams of words, so just take the first
        // anagram (the unique alphabet bit-pattern).
        //
        if seen.insert(bits) {
            words.push(zwrd);
        }
    }

    freq.sort_unstable_by_key(|x| x.1);

    if cfg!(debug_assertions) {
        // print letter frequencies
        //
        for fp in freq {
            println!("{}: {}", fp.0, fp.1);
        }
    }

    // build bitmask LUT from frequencies. The idea is that each
    // character gets assigned a new bit position, based upon its
    // frequency in the valid words.
    //
    // eg:
    //   ('q' x 100) : mask_lut[0] = (0b...0000_0000_0000_0001, 0)
    //   ('x' x 310) : mask_lut[4] = (0b...0000_0000_0000_0010, 1)
    //   ('j' x 350) : mask_lut[8] = (0b...0000_0000_0000_0100, 2)
    //
    let mut mask_lut: [(u32, usize); 26] = Default::default();

    for (i, &(z, _)) in freq.iter().enumerate() {
        mask_lut[z.ord()] = (1u32 << i, i);
    }

    // give each word a new mask, where the least-frequent letters
    // appear closer to the LSB (least significant bit) in the
    // bitfield.
    //
    // eg: "cats" (numbers are invented, and not representative)
    //
    // ('c' x 989) = 0b...0000_0100_0000_0000
    // ('a' x 100) = 0b...0000_0000_0000_0001  < least freq' so LSB
    // ('t' x 340) = 0b...0000_0000_0100_0000
    // ('s' x 123) = 0b...0000_0000_0000_1000
    //
    // We also stick all words with the same LSB into a bucket, so
    // we can easily look them up. This means we can EFFICIENTLY
    // fil a target bit-pattern quickly.
    //
    let mut lbit_lut: [Vec<u32>; 26] = Default::default();
    let mut word_lut: HashMap<u32, ZWord> = Default::default();

    for word in words {
        let mut new_bits = 0;
        let mut lowbit = 26;

        for z in *word {
            let idx = z.ord();
            let msk = mask_lut[idx].0;
            let lsb = mask_lut[idx].1;

            new_bits |= msk;

            lowbit = lowbit.min(lsb);
        }

        lbit_lut[lowbit].push(new_bits);
        word_lut.insert(new_bits, word);
    }

    // do the search, trying to fill our first free bit in our
    // final 'mask', using the LSB lookups.
    //
    fn search(
        selected: &mut [u32; 5],
        lut: &[Vec<u32>; 26],
        mask: u32,
        depth: usize,
        word_lut: &HashMap<u32, ZWord>,
    ) {
        if depth == 5 {
            println!(
                "{} {} {} {} {}",
                word_lut[&selected[0]],
                word_lut[&selected[1]],
                word_lut[&selected[2]],
                word_lut[&selected[3]],
                word_lut[&selected[4]]
            );
            return;
        }

        // find the lowest free bit (next low-frequency character)
        //
        let lowbit = mask.trailing_ones();
        let words = &lut[lowbit as usize];

        if cfg!(debug_assertions) {
            println!(
                "free lowbit [{:#02}] with mask [{:#028b}] at depth {} :: searching {} words...",
                lowbit,
                mask,
                depth,
                words.len()
            );
        }

        for &bits in words {
            if mask & bits == 0 {
                selected[depth] = bits;
                search(selected, lut, mask | bits, depth + 1, word_lut);
            }
        }
    }

    (0..27).into_par_iter().for_each(|i| {
        let mask = 1 << i;
        let mut selected: [u32; 5] = Default::default();

        search(&mut selected, &lbit_lut, mask, 0, &word_lut);
    });
}

const U8A: u8 = 'a' as u8;

#[derive(Default, Copy, Clone)]
struct ZChar(u8);

impl ZChar {
    fn from(c: char) -> Self {
        ZChar((c.to_ascii_lowercase() as u8) - U8A)
    }

    fn chr(&self) -> char {
        (&self.0 + U8A) as char
    }

    fn mask(&self) -> u32 {
        1 << self.0
    }

    fn ord(&self) -> usize {
        self.0 as usize
    }
}

impl Display for ZChar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.chr())
    }
}

#[derive(Default, Copy, Clone)]
struct ZWord([ZChar; 5]);

impl Deref for ZWord {
    type Target = [ZChar; 5];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ZWord {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for ZWord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}{}",
            self.0[0].chr(),
            self.0[1].chr(),
            self.0[2].chr(),
            self.0[3].chr(),
            self.0[4].chr(),
        )
    }
}
