//! md-viewer patch: the look a variation selector asks of an emoji.
//!
//! epaint lays text out one character at a time and picks the face of each
//! on its own. A variation selector after a character says which of its two
//! looks is meant, so layout looks one character ahead for it.

/// U+FE0E, which asks for the character before it in the ink of the text.
const TEXT_SELECTOR: char = '\u{FE0E}';

/// U+FE0F, which asks for the character before it as a color emoji.
const EMOJI_SELECTOR: char = '\u{FE0F}';

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

#[cfg(test)]
mod tests {
    use super::*;

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
