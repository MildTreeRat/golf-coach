//! CPython 3.13's `textwrap.fill`, to the character, for the caveat prose. [M29 P9]
//!
//! Written in `crates/mcp` and moved here at M29 P26, when `golf-lab analyze`'s report wanted
//! `textwrap.wrap(width=88)` for the coaching paragraph: `golf-mcp` and `golf-lab` are shells that
//! may not depend on each other, and both depend on `golf-core`, so one definition serves both
//! (`golf_mcp::textwrap` re-exports it, and P28's flight report reaches it the same way).
//!
//! `contracts/caveats.py` wraps its derived bullets with `textwrap.fill(width=95)`, and the wrapped
//! text is what the surface case recorded, so a derived clause that broke one word earlier would
//! move every line after it. This is `TextWrapper` with the defaults those calls use: tabs expanded
//! at 8, each whitespace character replaced by a space, words broken at hyphens, long words broken,
//! whitespace dropped at line ends, no sentence-ending fix and no line limit.
//!
//! Here rather than in `pyfmt`, which is where CPython's formatting edges live, because every one of
//! those is gated by a `spec/vectors/format/` table and the M29 plan leaves that family untouched.
//! The tests below are frozen Python's own output on the shapes the regex treats specially.
//!
//! # The chunker is `wordsep_re`, read as code
//!
//! `TextWrapper._split` splits on one regex whose alternatives are tried at each position in order:
//! a whitespace run; an em-dash `--` (two or more hyphens after a word character and before one);
//! and a lazily-grown word that ends at a breakable hyphen (two letters before it, or
//! letter-hyphen-letter, and a letter, an optional hyphen and a letter after it), at whitespace or
//! the end, or before an em-dash. [`split`] grows the word one character at a time and tries those
//! three ends in that order, which is what the lazy quantifier does. Its lookbehinds may see before
//! the chunk's own start, as the regex's do.
//!
//! **Python's `\w` and `\d` are Unicode classes and these are close, not equal**: a word character
//! is `char::is_alphanumeric` or `_`, and a digit is an ASCII digit or any non-ASCII numeric, where
//! Python's `\d` is category Nd alone. They agree on every character the caveats hold (ASCII, `—`
//! and `±`), and a prose change that reaches a character where they part would show as a surface
//! difference, not as silence.

/// `textwrap._whitespace`: the six characters the regex calls whitespace. Not Unicode's set.
const WHITESPACE: [char; 6] = ['\t', '\n', '\x0b', '\x0c', '\r', ' '];

fn is_ws(c: char) -> bool {
    WHITESPACE.contains(&c)
}

/// `\w`, as the module doc says.
fn is_word(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// `\d`, as the module doc says.
fn is_digit(c: char) -> bool {
    c.is_ascii_digit() || (!c.is_ascii() && c.is_numeric())
}

/// The regex's `letter`, `[^\d\W]`: a word character that is not a digit, so `_` is one.
fn is_letter(c: char) -> bool {
    is_word(c) && !is_digit(c)
}

/// The regex's `word_punct`, `[\w!"'&.,?]`.
fn is_word_punct(c: char) -> bool {
    is_word(c) || matches!(c, '!' | '"' | '\'' | '&' | '.' | ',' | '?')
}

/// `str.strip() == ''`: blank by Python's `str.isspace`, which adds the four information separators
/// to Unicode's white space.
fn is_blank(chunk: &[char]) -> bool {
    chunk
        .iter()
        .all(|&c| c.is_whitespace() || ('\x1c'..='\x1f').contains(&c))
}

/// `str.expandtabs(8)`: the column restarts after `\n` and `\r`.
fn expand_tabs(text: &str) -> Vec<char> {
    let mut out = Vec::with_capacity(text.len());
    let mut column = 0usize;
    for c in text.chars() {
        match c {
            '\t' => {
                let pad = 8 - column % 8;
                out.extend(std::iter::repeat_n(' ', pad));
                column += pad;
            }
            '\n' | '\r' => {
                out.push(c);
                column = 0;
            }
            _ => {
                out.push(c);
                column += 1;
            }
        }
    }
    out
}

/// `_munge_whitespace`: tabs expanded, then every whitespace character a space.
fn munge(text: &str) -> Vec<char> {
    expand_tabs(text)
        .into_iter()
        .map(|c| if is_ws(c) { ' ' } else { c })
        .collect()
}

/// `-{2,}(?=\w)` at `at`: the end of the hyphen run when it is two or more long and a word
/// character follows it. Greedy, and backing off cannot help: a shorter run is followed by `-`.
fn em_dash_end(s: &[char], at: usize) -> Option<usize> {
    let mut end = at;
    while end < s.len() && s[end] == '-' {
        end += 1;
    }
    (end - at >= 2 && end < s.len() && is_word(s[end])).then_some(end)
}

/// Whether the hyphen at `at` ends a hyphenated word's chunk:
/// `-(?:(?<=lt{2}-)|(?<=lt-lt-))(?=lt-?lt)`.
fn breaks_after_hyphen(s: &[char], at: usize) -> bool {
    let letter = |i: usize| i < s.len() && is_letter(s[i]);
    let behind = (at >= 2 && letter(at - 2) && letter(at - 1))
        || (at >= 3 && letter(at - 3) && s[at - 2] == '-' && letter(at - 1));
    let ahead = letter(at + 1)
        && ((at + 2 < s.len() && s[at + 2] == '-' && letter(at + 3)) || letter(at + 2));
    behind && ahead
}

/// Where the word chunk starting at `start` ends: the lazy `\S+?` followed by its three ends.
fn word_end(s: &[char], start: usize) -> usize {
    let mut end = start + 1;
    loop {
        if end < s.len() && s[end] == '-' && breaks_after_hyphen(s, end) {
            return end + 1;
        }
        if end == s.len() || is_ws(s[end]) {
            return end;
        }
        if is_word_punct(s[end - 1]) && em_dash_end(s, end).is_some() {
            return end;
        }
        end += 1;
    }
}

/// `TextWrapper._split` on munged text: every chunk, whitespace runs included, in order.
fn split(s: &[char]) -> Vec<Vec<char>> {
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < s.len() {
        let end = if is_ws(s[start]) {
            start + s[start..].iter().take_while(|&&c| is_ws(c)).count()
        } else if let Some(end) = (start > 0 && is_word_punct(s[start - 1]))
            .then(|| em_dash_end(s, start))
            .flatten()
        {
            end
        } else {
            word_end(s, start)
        };
        chunks.push(s[start..end].to_vec());
        start = end;
    }
    chunks
}

/// `_handle_long_word` with `break_long_words` and `break_on_hyphens` on: cut the next chunk to the
/// space left, at its last hyphen inside that space when one follows something other than hyphens.
fn handle_long_word(
    reversed: &mut [Vec<char>],
    line: &mut Vec<Vec<char>>,
    line_len: isize,
    width: isize,
) {
    let space_left = if width < 1 { 1 } else { width - line_len } as usize;
    let chunk = reversed.last_mut().expect("a chunk to break");
    let mut end = space_left.min(chunk.len());
    if chunk.len() > space_left {
        if let Some(hyphen) = chunk[..space_left].iter().rposition(|&c| c == '-') {
            if hyphen > 0 && chunk[..hyphen].iter().any(|&c| c != '-') {
                end = hyphen + 1;
            }
        }
    }
    line.push(chunk[..end].to_vec());
    chunk.drain(..end);
}

/// `textwrap.wrap(text, width, initial_indent=…, subsequent_indent=…)`: the lines, unjoined.
pub fn wrap(
    text: &str,
    width: usize,
    initial_indent: &str,
    subsequent_indent: &str,
) -> Vec<String> {
    assert!(width > 0, "invalid width {width} (must be > 0)");
    let mut reversed = split(&munge(text));
    reversed.reverse();
    let mut lines: Vec<String> = Vec::new();
    while !reversed.is_empty() {
        let indent = if lines.is_empty() {
            initial_indent
        } else {
            subsequent_indent
        };
        let width = width as isize - indent.chars().count() as isize;
        if !lines.is_empty() && reversed.last().is_some_and(|c| is_blank(c)) {
            reversed.pop();
        }
        let mut line: Vec<Vec<char>> = Vec::new();
        let mut line_len: isize = 0;
        while let Some(next) = reversed.last() {
            let len = next.len() as isize;
            if line_len + len > width {
                break;
            }
            line.push(reversed.pop().expect("the chunk just read"));
            line_len += len;
        }
        if reversed.last().is_some_and(|c| c.len() as isize > width) {
            handle_long_word(&mut reversed, &mut line, line_len, width);
        }
        if line.last().is_some_and(|c| is_blank(c)) {
            line.pop();
        }
        if !line.is_empty() {
            let body: String = line.iter().flatten().collect();
            lines.push(format!("{indent}{body}"));
        }
    }
    lines
}

/// `textwrap.fill`: [`wrap`]'s lines joined by `\n`, with no newline after the last.
pub fn fill(text: &str, width: usize, initial_indent: &str, subsequent_indent: &str) -> String {
    wrap(text, width, initial_indent, subsequent_indent).join("\n")
}

#[cfg(test)]
mod tests {
    use super::fill;

    /// Frozen Python 3.13.3's `textwrap.fill` on each, printed at M29 P9: hyphenated words, the
    /// em-dash rules, long words with and without hyphens, tabs and newlines, a leading space kept
    /// on the first line only, underscores as letters, digits that do not break, `—`, and runs of
    /// hyphens alone.
    const PYTHON: &[(&str, usize, &str, &str, &str)] = &[
        (
            "re-film the swing, steady the camera or shoot it again",
            12,
            "",
            "",
            "re-film the\nswing,\nsteady the\ncamera or\nshoot it\nagain",
        ),
        (
            "a 2-D-per-view reading, hand-annotated and down-the-line",
            14,
            "- ",
            "  ",
            "- a 2-D-per-\n  view\n  reading,\n  hand-\n  annotated\n  and down-\n  the-line",
        ),
        (
            "well--known  em--dash and a---b and -- alone",
            10,
            "",
            "",
            "well--\nknown  em\n--dash and\na---b and\n-- alone",
        ),
        (
            "supercalifragilisticexpialidocious-word-with-hyphens end",
            15,
            "",
            "",
            "supercalifragil\nisticexpialidoc\nious-word-with-\nhyphens end",
        ),
        (
            "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            8,
            "- ",
            "  ",
            "- xxxxxx\n  xxxxxx\n  xxxxxx\n  xxxxxx\n  xxxxxx",
        ),
        (
            "tabs\there and\nnewlines\r\nand  double  spaces ",
            9,
            "",
            "",
            "tabs\nhere and\nnewlines\nand\ndouble\nspaces",
        ),
        (
            "  leading spaces kept on the first line only",
            10,
            "",
            "",
            "  leading\nspaces\nkept on\nthe first\nline only",
        ),
        (
            "`no_band`, `no_handedness` and `cross_view_contradicted` are not",
            20,
            "- ",
            "  ",
            "- `no_band`,\n  `no_handedness`\n  and `cross_view_co\n  ntradicted` are\n  not",
        ),
        (
            "x-ray e-mail co-op A-B-C 1-2 ab-cd-ef a-b-c-d",
            6,
            "",
            "",
            "x-ray\ne-mail\nco-op\nA-B-C\n1-2\nab-cd-\nef\na-b-\nc-d",
        ),
        (
            "the swing \u{2014} a 2-D projection \u{2014} foreshortened",
            11,
            "",
            "",
            "the swing \u{2014}\na 2-D\nprojection\n\u{2014} foreshort\nened",
        ),
        (
            "-leading and trailing- hyphens --flag",
            8,
            "",
            "",
            "-leading\nand trai\nling-\nhyphens\n--flag",
        ),
        (
            "---------- dashes only ----------",
            6,
            "",
            "",
            "------\n----\ndashes\nonly -\n------\n---",
        ),
    ];

    #[test]
    fn fill_is_frozen_pythons_on_every_shape_the_regex_treats_specially() {
        for &(text, width, initial, subsequent, python) in PYTHON {
            assert_eq!(
                fill(text, width, initial, subsequent),
                python,
                "{text:?} at {width}"
            );
        }
    }

    #[test]
    fn nothing_to_wrap_is_no_lines() {
        assert_eq!(fill("", 95, "- ", "  "), "");
        assert_eq!(fill("   ", 95, "- ", "  "), "");
    }
}
