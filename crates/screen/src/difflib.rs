//! CPython's `difflib.SequenceMatcher(None, a, b).ratio()`, reproduced to the bit. [M34 P3]
//!
//! Every tile the parser locates, it locates by this number: `ProfileField::matches` scores a box's
//! text against a label with it, and the 0.8 threshold and `field_for`'s `>=` are both read off it.
//! So a ratio a last bit away from CPython's can move a box across the threshold, or hand a tie to
//! the other field, and the parse that follows is a different parse. It is ported rather than
//! replaced with a textbook similarity for that reason: `ratio` is not edit distance, and not even
//! the longest common subsequence. It is whatever CPython's greedy block search finds, and that
//! search is the definition.
//!
//! The gate is `spec/vectors/format/difflib_ratio.json`, recorded by CPython and run by
//! `tests/difflib.rs` on bits. Its inputs are every box text from M34's planning run against every
//! label and alias of the profile, normalized as the parser normalizes them, plus crafted cases for
//! the three places a plausible port goes wrong.
//!
//! # The three places
//!
//! - **`find_longest_match`'s tie-break.** Of the longest blocks it returns the one starting
//!   earliest in `a`, then earliest in `b`, which falls out of scanning `a` in order and taking a
//!   block only when it is *strictly* longer. The blocks found decide the recursion, so a port that
//!   keeps the last longest block can find fewer matches overall, not just different ones:
//!   `("aaa", "aaba")` is 6/7 in CPython and 4/7 with `>=`.
//! - **The extension step.** After the scan, the block is grown over equal elements on both sides.
//!   That is what re-finds a match across characters the scan never looked at, which matters only
//!   with the next item.
//! - **`autojunk`.** CPython defaults it on, and it drops from `b`'s index every element occurring
//!   more than `len(b) / 100 + 1` times — but only when `len(b) >= 200`. Every label here is short,
//!   so the parser never reaches it, and it is **ported anyway** rather than refused: it is six
//!   lines, and the table carries three long cases from M34 P2, one of which it changes
//!   (`("QQQ", "XYZ" + "Q" * 197)` is `0.0` with it and `0.0296` without). In `("AB", "AB" * 100)`
//!   both letters are popular and the scan finds nothing, and the extension step still grows the
//!   empty block to `AB`, because it refuses only junk, and popular is not junk.
//!
//! `isjunk` is always `None` here, so nothing is junk. CPython's second pair of extension loops,
//! which absorb *junk* on either side of a block, never run without it and are not ported.
//!
//! Sequences are code points, as a Python `str` is, so a `°` or an `ē` counts as one element and the
//! ratio's denominator is a count of `char`s, never of UTF-8 bytes.

use std::collections::HashMap;

/// `SequenceMatcher(None, a, b).ratio()`: twice the matched elements over the total.
pub fn ratio(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let matcher = Matcher::new(&a, &b);
    let matches: usize = matcher
        .matching_blocks()
        .iter()
        .map(|block| block.size)
        .sum();
    calculate_ratio(matches, a.len() + b.len())
}

/// `_calculate_ratio`. `2.0 * matches` is exact and the division is one correctly rounded IEEE
/// operation, as in Python, so the bits agree; two empty strings are identical, so `1.0`.
fn calculate_ratio(matches: usize, length: usize) -> f64 {
    if length == 0 {
        return 1.0;
    }
    2.0 * matches as f64 / length as f64
}

/// `difflib.Match`: `a[a..a + size] == b[b..b + size]`. The field order is the tuple's, so the
/// derived `Ord` sorts the blocks as Python sorts the tuples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Match {
    a: usize,
    b: usize,
    size: usize,
}

struct Matcher<'s> {
    a: &'s [char],
    b: &'s [char],
    /// Every element of `b` to the indices it occurs at, ascending, less the popular ones.
    b2j: HashMap<char, Vec<usize>>,
}

impl<'s> Matcher<'s> {
    /// `__init__` with `isjunk=None` and `autojunk=True`, then `__chain_b`.
    fn new(a: &'s [char], b: &'s [char]) -> Self {
        let mut b2j: HashMap<char, Vec<usize>> = HashMap::new();
        for (index, &element) in b.iter().enumerate() {
            b2j.entry(element).or_default().push(index);
        }
        // `autojunk`: the threshold and the `>` are CPython's. Which order the popular elements are
        // dropped in cannot matter, so a `retain` stands in for its two loops.
        let n = b.len();
        if n >= 200 {
            let ntest = n / 100 + 1;
            b2j.retain(|_, indices| indices.len() <= ntest);
        }
        Self { a, b, b2j }
    }

    /// The longest block in `a[alo..ahi]` and `b[blo..bhi]`, earliest in `a` and then in `b` on a
    /// tie, after the extension step has grown it. When the scan finds nothing its block is the
    /// empty one at `(alo, blo)`, which the extension step can still grow.
    fn find_longest_match(&self, alo: usize, ahi: usize, blo: usize, bhi: usize) -> Match {
        let (a, b) = (self.a, self.b);
        let (mut best_i, mut best_j, mut best_size) = (alo, blo, 0);
        // `j2len[j]` is the length of the longest match ending with `a[i - 1]` and `b[j]`. A map
        // rather than a row of `bhi` slots, as CPython keeps it: only the previous row is ever
        // read, and only at the `j`s that row wrote.
        let mut j2len: HashMap<usize, usize> = HashMap::new();
        for (i, element) in a.iter().enumerate().take(ahi).skip(alo) {
            let mut next: HashMap<usize, usize> = HashMap::new();
            if let Some(indices) = self.b2j.get(element) {
                for &j in indices {
                    if j < blo {
                        continue;
                    }
                    if j >= bhi {
                        break;
                    }
                    let k = j
                        .checked_sub(1)
                        .and_then(|previous| j2len.get(&previous))
                        .copied()
                        .unwrap_or(0)
                        + 1;
                    next.insert(j, k);
                    // Strictly longer, which is the whole tie-break.
                    if k > best_size {
                        (best_i, best_j, best_size) = (i + 1 - k, j + 1 - k, k);
                    }
                }
            }
            j2len = next;
        }
        // The extension step. With no junk, CPython's `not isbjunk(...)` is always true.
        while best_i > alo && best_j > blo && a[best_i - 1] == b[best_j - 1] {
            (best_i, best_j, best_size) = (best_i - 1, best_j - 1, best_size + 1);
        }
        while best_i + best_size < ahi
            && best_j + best_size < bhi
            && a[best_i + best_size] == b[best_j + best_size]
        {
            best_size += 1;
        }
        Match {
            a: best_i,
            b: best_j,
            size: best_size,
        }
    }

    /// `get_matching_blocks`: the blocks, sorted, adjacent ones collapsed, and the `(len(a),
    /// len(b), 0)` sentinel last.
    ///
    /// CPython's queue is a stack (`pop()` takes the last), and the order it pops in cannot change
    /// which blocks are found: each popped range is disjoint from every other, and what it yields
    /// depends only on its own bounds. It is kept anyway, so this reads beside `difflib.py` line for
    /// line. The collapse cannot change a ratio either, only how its total is split.
    fn matching_blocks(&self) -> Vec<Match> {
        let (la, lb) = (self.a.len(), self.b.len());
        let mut queue = vec![(0, la, 0, lb)];
        let mut blocks = Vec::new();
        while let Some((alo, ahi, blo, bhi)) = queue.pop() {
            let block = self.find_longest_match(alo, ahi, blo, bhi);
            if block.size > 0 {
                blocks.push(block);
                if alo < block.a && blo < block.b {
                    queue.push((alo, block.a, blo, block.b));
                }
                if block.a + block.size < ahi && block.b + block.size < bhi {
                    queue.push((block.a + block.size, ahi, block.b + block.size, bhi));
                }
            }
        }
        blocks.sort();

        let mut collapsed = Vec::with_capacity(blocks.len() + 1);
        let mut current = Match {
            a: 0,
            b: 0,
            size: 0,
        };
        for block in blocks {
            if current.a + current.size == block.a && current.b + current.size == block.b {
                current.size += block.size;
            } else {
                if current.size > 0 {
                    collapsed.push(current);
                }
                current = block;
            }
        }
        if current.size > 0 {
            collapsed.push(current);
        }
        collapsed.push(Match {
            a: la,
            b: lb,
            size: 0,
        });
        collapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    fn block(a: usize, b: usize, size: usize) -> Match {
        Match { a, b, size }
    }

    /// `find_longest_match`'s own docstring: of the two five-long blocks, the one at `b = 4` wins
    /// over a shorter one at `b = 0`, and the extension step grows the scan's four to five.
    #[test]
    fn the_longest_block_is_found_as_cpythons_docstring_says() {
        let (a, b) = (chars(" abcd"), chars("abcd abcd"));
        assert_eq!(
            Matcher::new(&a, &b).find_longest_match(0, 5, 0, 9),
            block(0, 4, 5)
        );
    }

    /// A tie between equally long blocks goes to the earliest in `a`, then the earliest in `b`.
    #[test]
    fn a_tie_goes_to_the_earliest_block() {
        let (a, b) = (chars("xyab"), chars("abxy"));
        assert_eq!(
            Matcher::new(&a, &b).find_longest_match(0, 4, 0, 4),
            block(0, 2, 2)
        );
        let (a, b) = (chars("ab"), chars("abab"));
        assert_eq!(
            Matcher::new(&a, &b).find_longest_match(0, 2, 0, 4),
            block(0, 0, 2)
        );
    }

    /// `get_matching_blocks`' own docstring, sentinel included, and its empty case.
    #[test]
    fn the_blocks_are_cpythons() {
        let (a, b) = (chars("abxcd"), chars("abcd"));
        assert_eq!(
            Matcher::new(&a, &b).matching_blocks(),
            [block(0, 0, 2), block(3, 2, 2), block(5, 4, 0)]
        );
        let (a, b) = (chars("ab"), chars("c"));
        assert_eq!(Matcher::new(&a, &b).matching_blocks(), [block(2, 1, 0)]);
    }

    /// The extension step growing a block **backwards** over a popular element, which the
    /// `difflib_ratio` table does not reach: with the backward loop removed every case in it still
    /// passes (M34 P3 measured that). `Q` occurs 197 times in a 200-long `b`, so the scan cannot see
    /// it and finds `AB`; only the extension recovers the `Q` before it. CPython 3.13.3's answer.
    #[test]
    fn the_extension_step_grows_a_block_backwards_over_a_popular_element() {
        let (a, b) = (chars("QAB"), chars(&format!("XQAB{}", "Q".repeat(196))));
        assert_eq!(
            Matcher::new(&a, &b).matching_blocks(),
            [block(0, 1, 3), block(3, 200, 0)]
        );
        assert_eq!(
            ratio("QAB", &format!("XQAB{}", "Q".repeat(196))),
            6.0 / 203.0
        );
    }

    #[test]
    fn the_ratio_is_twice_the_matches_over_the_total() {
        assert_eq!(ratio("abcd", "bcde"), 0.75);
        assert_eq!(ratio("", ""), 1.0);
        assert_eq!(ratio("", "CARRY"), 0.0);
        // The tie-break's consequence: keeping the *last* longest block scores this 4/7.
        assert_eq!(ratio("aaa", "aaba"), 6.0 / 7.0);
        // Code points, not bytes: `°` is two bytes and one element.
        assert_eq!(ratio("\u{b0}C", "C"), 2.0 / 3.0);
    }
}
