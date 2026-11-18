//! md-viewer patch: where an emoji sequence starts and ends in text.
//!
//! epaint lays text out one character at a time, but some emoji are written
//! as several: a sign and a variation selector (❤️), two regional indicators
//! (🇯🇵), a hand and a skin tone (👍🏽), a digit and a keycap (1️⃣), or people
//! held together by zero-width joiners (👨‍👩‍👧). Layout asks this module how
//! far such a sequence reaches, and then asks the faces for one glyph that
//! draws the whole of it.

/// U+FE0E, which asks for the character before it in the ink of the text.
const TEXT_SELECTOR: char = '\u{FE0E}';

/// U+FE0F, which asks for the character before it as a color emoji.
pub(super) const EMOJI_SELECTOR: char = '\u{FE0F}';

const ZERO_WIDTH_JOINER: char = '\u{200D}';

/// No sequence in Unicode's emoji lists is half this long. The limit keeps a
/// long run of joined characters from being scanned again from each of them.
const MAX_SEQUENCE_BYTES: usize = 64;

/// The look a variation selector asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Presentation {
    /// From the color emoji face.
    Emoji,

    /// From the text faces, in the ink of the text.
    Text,
}

impl Presentation {
    /// The look `selector` asks of the character before it, if it is one of
    /// the two emoji variation selectors.
    pub(super) fn asked_by(selector: char) -> Option<Self> {
        match selector {
            EMOJI_SELECTOR => Some(Self::Emoji),
            TEXT_SELECTOR => Some(Self::Text),
            _ => None,
        }
    }
}

/// Whether `next`, the character after `c`, may make the two part of one
/// emoji sequence. Layout asks this of every pair of characters, so it is
/// only a few comparisons, and text without emoji never gets past it.
#[inline]
pub(super) fn may_join(c: char, next: char) -> bool {
    if is_regional_indicator(c) {
        return is_regional_indicator(next);
    }
    (extends(next) || next == ZERO_WIDTH_JOINER) && may_start(c, next)
}

/// Whether `c` followed by `next` can be the start of an emoji sequence.
/// Letters never are, so a joiner inside Arabic or Devanagari is left alone;
/// a digit, `#` and `*` are only as the base of a keycap.
fn may_start(c: char, next: char) -> bool {
    match c {
        '#' | '*' | '0'..='9' => matches!(next, TEXT_SELECTOR | EMOJI_SELECTOR | '\u{20E3}'),
        '\u{A9}' | '\u{AE}' => true,
        _ => c >= '\u{203C}' && !extends(c),
    }
}

/// Characters that change the emoji before them instead of standing alone:
/// the two selectors, the combining keycap, the five skin tones and the tag
/// characters that spell a regional flag.
fn extends(c: char) -> bool {
    matches!(
        c,
        TEXT_SELECTOR
            | EMOJI_SELECTOR
            | '\u{20E3}'
            | '\u{1F3FB}'..='\u{1F3FF}'
            | '\u{E0020}'..='\u{E007F}'
    )
}

fn is_regional_indicator(c: char) -> bool {
    matches!(c, '\u{1F1E6}'..='\u{1F1FF}')
}

/// Bytes of the first character of `text` and of what extends it.
fn element_len(text: &str) -> usize {
    let mut chars = text.chars();
    let Some(base) = chars.next() else {
        return 0;
    };
    let extenders = chars.take_while(|&c| extends(c)).map(char::len_utf8);
    base.len_utf8() + extenders.sum::<usize>()
}

/// Bytes of the emoji sequence `text` starts with: a pair of regional
/// indicators, or a character with what extends it, and each further one a
/// zero-width joiner ties on. It is the first character alone when nothing
/// after it belongs to it.
///
/// This is wider than Unicode's lists of sequences: it tells layout what to
/// ask the faces about, and the faces decide what is drawn as one glyph.
pub(super) fn sequence_len(text: &str) -> usize {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return 0;
    };
    if is_regional_indicator(first) {
        let second = chars.next().filter(|&c| is_regional_indicator(c));
        return first.len_utf8() + second.map_or(0, char::len_utf8);
    }

    let mut len = element_len(text);
    while len < MAX_SEQUENCE_BYTES {
        let Some(joined) = text[len..].strip_prefix(ZERO_WIDTH_JOINER) else {
            break;
        };
        match joined.chars().next() {
            Some(c) if may_start(c, ZERO_WIDTH_JOINER) => {
                len += ZERO_WIDTH_JOINER.len_utf8() + element_len(joined);
            }
            _ => break,
        }
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sequence_reaches_as_far_as_its_last_joined_character() {
        for (text, sequence) in [
            ("\u{2764}\u{FE0F} x", "\u{2764}\u{FE0F}"),
            ("\u{260E}\u{FE0E}x", "\u{260E}\u{FE0E}"),
            ("\u{1F1EF}\u{1F1F5}\u{1F1FA}\u{1F1F8}", "\u{1F1EF}\u{1F1F5}"),
            ("\u{1F1EF}x", "\u{1F1EF}"),
            ("\u{1F44D}\u{1F3FD}\u{1F44D}", "\u{1F44D}\u{1F3FD}"),
            ("1\u{FE0F}\u{20E3}2", "1\u{FE0F}\u{20E3}"),
            (
                "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}!",
                "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
            ),
            (
                "\u{2764}\u{FE0F}\u{200D}\u{1F525}\u{1F525}",
                "\u{2764}\u{FE0F}\u{200D}\u{1F525}",
            ),
            (
                "\u{1F3F4}\u{E0067}\u{E0062}\u{E007F} ",
                "\u{1F3F4}\u{E0067}\u{E0062}\u{E007F}",
            ),
            // A joiner with nothing to join on to is not part of the emoji.
            ("\u{1F468}\u{200D}", "\u{1F468}"),
            ("\u{1F468}\u{200D}a", "\u{1F468}"),
            ("\u{1F468}\u{200D}1", "\u{1F468}"),
            ("\u{1F468}\u{1F469}", "\u{1F468}"),
            ("ab", "a"),
            ("", ""),
        ] {
            assert_eq!(&text[..sequence_len(text)], sequence, "{text:?}");
        }
    }

    #[test]
    fn only_emoji_start_a_sequence() {
        for (c, next) in [
            ('\u{2764}', '\u{FE0F}'),
            ('\u{1F600}', '\u{FE0E}'),
            ('\u{1F1EF}', '\u{1F1F5}'),
            ('\u{1F44D}', '\u{1F3FD}'),
            ('1', '\u{FE0F}'),
            ('#', '\u{20E3}'),
            ('\u{1F468}', '\u{200D}'),
            ('\u{A9}', '\u{FE0F}'),
        ] {
            assert!(may_join(c, next), "{c:?} {next:?}");
        }
        for (c, next) in [
            ('a', 'b'),
            ('a', '\u{FE0F}'),
            ('1', '\u{200D}'),
            ('1', '\u{1F3FD}'),
            // A joiner between letters shapes the script, not an emoji.
            ('\u{0915}', '\u{200D}'),
            ('\u{0644}', '\u{200D}'),
            ('\u{1F468}', '\u{1F469}'),
            ('\u{1F1EF}', 'a'),
            ('\u{FE0F}', '\u{FE0F}'),
        ] {
            assert!(!may_join(c, next), "{c:?} {next:?}");
        }
    }

    #[test]
    fn a_long_run_of_joined_emoji_is_cut_short() {
        let run = "\u{1F600}\u{200D}".repeat(1000);
        let len = sequence_len(&run);
        assert!(len >= MAX_SEQUENCE_BYTES, "{len}");
        assert!(len < 2 * MAX_SEQUENCE_BYTES, "{len}");
    }

    #[test]
    fn the_two_emoji_selectors_ask_for_a_look() {
        assert_eq!(
            Presentation::asked_by('\u{FE0F}'),
            Some(Presentation::Emoji)
        );
        assert_eq!(Presentation::asked_by('\u{FE0E}'), Some(Presentation::Text));
        assert_eq!(Presentation::asked_by('\u{FE00}'), None);
    }
}
