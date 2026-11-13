color-emoji-subset.ttf
======================

Test fixture for src/system_fonts/emoji/tests.rs. It lets the color-emoji
tests run without a color emoji font installed on the machine.

It is a subset of Noto Color Emoji 2.047 (noto-emoji 20240827, CBDT/CBLC
bitmaps), Copyright 2022 Google Inc., licensed under the SIL Open Font
License, Version 1.1. The copyright and license strings are kept in the
font's name table.

Glyphs kept: U+1F7E2, U+1F534, U+1F535, U+26AA, U+2705 (emoji the tests
render) and U+25B6, U+26A0, U+2714 (UI symbols the font also maps).

Regenerate with fonttools:

    pyftsubset NotoColorEmoji.ttf \
        --unicodes=U+1F7E2,U+1F534,U+1F535,U+26AA,U+2705,U+25B6,U+25BC,U+26A0,U+2714,U+2192,U+2713 \
        --no-layout-closure --layout-features= --name-IDs='*' --notdef-outline \
        --output-file=color-emoji-subset.ttf

U+25BC, U+2192 and U+2713 are requested but absent from the source font.
