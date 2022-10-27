#![feature(portable_simd)]

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::simd::{Simd, SimdPartialEq};
use std::str::from_utf8;
use std::time::{Duration, Instant};

use anyhow::{ensure, Result};
use itertools::Itertools;
use memmap::Mmap;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefMutIterator, ParallelDrainRange, ParallelExtend,
    ParallelIterator,
};

const MAX_LETTERS: usize = 26;
const SIMD_LANES: usize = 64;

fn main() -> Result<()> {
    let path = env::args().nth(1).expect("No input.");
    let expect_word_count = env::args()
        .nth(2)
        .unwrap_or_else(|| "5".to_string())
        .parse::<usize>()?;
    let expect_word_length = env::args()
        .nth(3)
        .unwrap_or_else(|| "5".to_string())
        .parse::<usize>()?;

    let expect_total_length = expect_word_length * expect_word_count;
    ensure!(
        expect_total_length <= MAX_LETTERS && expect_total_length > 0,
        "Invalid numbers."
    );
    let allowed_skips = MAX_LETTERS - expect_total_length;

    let time = Instant::now();

    let mmap = unsafe { Mmap::map(&File::open(path)?)? };

    let mut bitmask_to_index = HashMap::<u32, usize>::new();
    let mut index_to_anagram = HashMap::<usize, Vec<&[u8]>>::new();

    let mut bitmask_list = Vec::new();
    let mut word_list = Vec::new();

    let mut char_rarity_bitmask_list: [Vec<u32>; MAX_LETTERS] = Default::default();
    let mut rarity_mask_list: [Vec<u32>; MAX_LETTERS] = Default::default();
    let mut rarity_mask_to_bitmask = HashMap::<u32, u32>::new();

    let mut word_offset = 0;
    let mut word_bitmask = 0;

    let mut char_frequency = [[0; 2]; MAX_LETTERS];
    for (i, char) in char_frequency.iter_mut().enumerate() {
        char[0] = i;
    }

    for (i, &char) in mmap.iter().enumerate() {
        if char != b'\r' && char != b'\n' {
            word_bitmask |= 1_u32 << (char - b'a');
            continue;
        }

        let word_length = i - word_offset;
        let word_bitmask_now = word_bitmask;

        word_offset = i + 1;
        word_bitmask = 0;

        if word_length != expect_word_length
            || word_bitmask_now.count_ones() as usize != expect_word_length
        {
            continue;
        }

        let word = &mmap[(i - expect_word_length)..i];

        if let Some(index) = bitmask_to_index.get(&word_bitmask_now) {
            if let Some(anagram) = index_to_anagram.get_mut(index) {
                anagram.push(word)
            } else {
                index_to_anagram.entry(*index).or_insert_with(|| vec![word]);
            }
            continue;
        }

        for char in word.iter() {
            let char_index = (char - b'a') as usize;
            char_frequency[char_index][1] += 1;
        }

        bitmask_to_index.insert(word_bitmask_now, word_list.len());
        bitmask_list.push(word_bitmask_now);
        word_list.push(word);
    }

    let duration_read = time.elapsed();
    let sys_time = Instant::now();

    let mut char_index_rarity: [usize; MAX_LETTERS] = Default::default();

    char_frequency // "qxjzvfwbkgpmhdcytlnuroisea"
        .iter()
        .sorted_by_key(|i| i[1])
        .enumerate()
        .for_each(|(index, item)| {
            char_index_rarity[item[0]] = index;
        });

    for i in bitmask_list {
        let rarity_mask = bitmask_to_raritymask(&i, &char_index_rarity);
        let min = rarity_mask.trailing_zeros() as usize;

        char_rarity_bitmask_list[min].push(i);
        rarity_mask_list[min].push(rarity_mask);
        rarity_mask_to_bitmask.insert(rarity_mask, i);
    }

    let duration_freq = sys_time.elapsed();
    let mut matching_stats = Vec::<(usize, Duration)>::new();
    let sys_time = Instant::now();

    let mut matching_list: [Vec<Entry>; MAX_LETTERS] = Default::default();
    matching_list[..(allowed_skips + 1)]
        .par_iter_mut()
        .zip(&rarity_mask_list)
        .enumerate()
        .for_each(|(char_index, (entry_list, raritymask_list))| {
            for rarity_mask in raritymask_list {
                (*entry_list).push(Entry::new(*rarity_mask, char_index));
            }
        });

    matching_stats.push((
        matching_list.iter().map(|list| list.len()).sum(),
        sys_time.elapsed(),
    ));
    let sys_time = Instant::now();

    let simd_zero_vec = Simd::<u32, SIMD_LANES>::splat(0);
    let simd_max_vec = Simd::<u32, SIMD_LANES>::splat(u32::MAX);
    let mut simd_idx = Simd::<usize, SIMD_LANES>::splat(0);
    for i in 0..simd_idx.lanes() {
        simd_idx[i] = i;
    }

    let mut depth = 1;
    while depth < expect_word_count {
        let sys_time = Instant::now();
        let solves = matching_list[..(MAX_LETTERS - expect_total_length + 1)]
            .par_iter_mut()
            .map(|entry_list| {
                let mut new_entry_list = Vec::new();
                let map = entry_list
                    .par_drain(..)
                    .flat_map(|mut word| {
                        let mut entry_matches = Vec::<Entry>::new();
                        let word_vec =
                            Simd::<u32, SIMD_LANES>::splat(word.rarity_mask);
                        loop {
                            if word.next_unset >= MAX_LETTERS {
                                break;
                            }

                            let mut entry_single_matches = /*if use_simd == 1 {*/
                                rarity_mask_list[word.next_unset]
                                    .chunks(SIMD_LANES)
                                    .filter_map(|rarity_mask_list| {
                                        let rarity_mask_list_vec =
                                            Simd::gather_or(rarity_mask_list, simd_idx, simd_max_vec);
                                        let new_rarity_mask_list_vec =
                                            rarity_mask_list_vec | word_vec;
                                        let matching_mask_vec =
                                            (rarity_mask_list_vec & word_vec)
                                                .simd_eq(simd_zero_vec);

                                        let matches = rarity_mask_list_vec.as_array().iter()
                                            .zip(new_rarity_mask_list_vec.as_array().iter())
                                            .zip(matching_mask_vec.to_array().iter())
                                            .filter(|(_, matching_mask)| {
                                                **matching_mask
                                            })
                                            .map(|((rarity_mask, new_rarity_mask), _)| {
                                                word.extend_unchecked(*rarity_mask, *new_rarity_mask)
                                            })
                                            .collect::<Vec<_>>();

                                        if matches.is_empty() {
                                            None
                                        } else {
                                            Some(matches)
                                        }
                                    })
                                    .flatten()
                                    .collect::<Vec<_>>();
                            /*} else {
                                rarity_mask_list[word.next_unset]
                                    .iter()
                                    .filter_map(|rarity_mask| word.extend(*rarity_mask))
                                    // .map(|word| {
                                    //     if depth == expect_word_count - 1 {
                                    //         println!("{:?}", word);
                                    //     };
                                    //     word
                                    // })
                                    .collect::<Vec<_>>()
                            };*/

                            if !entry_single_matches.is_empty() {
                                entry_matches.append(&mut entry_single_matches);
                            }

                            if word.skipped < allowed_skips {
                                word.skipped += 1;
                            } else {
                                break;
                            }

                            word.update_unset();
                        }

                        entry_matches
                    });

                new_entry_list.par_extend(map);

                // entry_list.clear();

                entry_list.extend(new_entry_list);

                entry_list.len()
            })
            .sum::<usize>();
        // .collect::<Vec<_>>();
        // solves.par_extend(map);

        // for (rarity, deque) in solves.iter_mut().enumerate() {
        //     matching_list[rarity].append(deque);
        // }
        //
        // println!(
        //     "Loop {}, matches {:?}, elapsed {:?}",
        //     depth,
        //     matching_list.iter().flatten().count(),
        //     sys_time.elapsed()
        // );

        depth += 1;
        matching_stats.push((solves, sys_time.elapsed()));
    }

    let duration_proc = sys_time.elapsed();

    let sys_time = Instant::now();
    let anagram_count = matching_list
        .iter()
        .flatten()
        .enumerate()
        .map(|(index, word)| {
            let index_map = word
                .rarity_mask_list
                .iter()
                .filter_map(|rarity_mask| rarity_mask_to_bitmask.get(rarity_mask))
                .filter_map(|bitmask| bitmask_to_index.get(bitmask));

            let words_string = index_map
                .clone()
                .filter_map(|&index| from_utf8(word_list[index]).ok())
                .join(" ");

            let anagram = index_map
                .filter_map(|index| index_to_anagram.get(index))
                .map(|words| words.len())
                .fold(1, |acc, x| acc * (x + 1))
                - 1;

            println!(
                "{} | {} | {:26b} | skipped: {}",
                words_string,
                index + 1,
                word.rarity_mask,
                word.skipped
            );

            anagram
        })
        .sum::<usize>();

    println!("{:-<60}", "-");
    for (depth, (matches, duration)) in matching_stats.iter().enumerate() {
        println!(
            "Loop depth: {}, matches: {:?}, elapsed: {:?}",
            depth, matches, duration
        );
    }

    if let Some((matches, _)) = matching_stats.last() {
        println!(
            "Matches with anagrams: {}, total matches: {}",
            anagram_count,
            anagram_count + matches
        );
    }
    let duration_print = sys_time.elapsed();
    println!("{:-<60}", "-");
    println!("File reading elapsed: {:?}", duration_read);
    println!("Frequency analysing elapsed: {:?}", duration_freq);
    println!("Matching elapsed: {:?}", duration_proc);
    println!("Printing elapsed: {:?}", duration_print);
    println!("Total elapsed: {:?}", time.elapsed());

    Ok(())
}

#[derive(Debug, Clone)]
struct Entry {
    rarity_mask_list: Vec<u32>,
    rarity_mask: u32,
    // rarity: usize,
    next_unset: usize,
    skipped: usize,
}

#[inline(always)]
fn bitmask_to_raritymask(bitmask: &u32, char_index_rarity: &[usize; 26]) -> u32 {
    let mut raritymask = 0;
    let mut bitmask = *bitmask;

    while bitmask != 0 {
        let char_index = bitmask.trailing_zeros();
        bitmask ^= 1_u32 << char_index;

        let char_rarity = char_index_rarity[char_index as usize];

        raritymask |= 1_u32 << char_rarity as u32;
    }

    raritymask
}

impl Entry {
    #[inline(always)]
    fn new(bitmask: u32, rarity: usize) -> Self {
        let mut entry = Self {
            rarity_mask_list: vec![bitmask],
            rarity_mask: bitmask,
            // rarity,
            next_unset: rarity,
            skipped: rarity,
        };

        entry.update_unset();

        entry
    }

    #[inline(always)]
    fn update_unset(&mut self) {
        let step = self.next_unset + 1;
        self.next_unset = (self.rarity_mask >> step).trailing_ones() as usize + step;
    }

    /*#[inline(always)]
    fn extend(&self, rarity_mask: u32) -> Option<Self> {
        if self.rarity_mask & rarity_mask == 0 {
            let mut new_entry = self.clone();
            new_entry.rarity_mask_list.push(rarity_mask);
            new_entry.rarity_mask |= rarity_mask;

            new_entry.update_unset();

            Some(new_entry)
        } else {
            None
        }
    }*/

    #[inline(always)]
    fn extend_unchecked(&self, rarity_mask: u32, new_rarity_mask: u32) -> Self {
        let mut new_entry = self.clone();
        new_entry.rarity_mask_list.push(rarity_mask);
        new_entry.rarity_mask = new_rarity_mask;

        new_entry.update_unset();

        new_entry
    }
}
