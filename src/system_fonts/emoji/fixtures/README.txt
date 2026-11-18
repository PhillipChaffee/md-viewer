color-emoji-subset.ttf
======================

Test fixture for src/system_fonts/emoji/tests.rs and for the layout tests of
vendor/epaint. It lets the color-emoji tests run without a color emoji font
installed on the machine.

It is a subset of Noto Color Emoji 2.047 (noto-emoji 20240827, CBDT/CBLC
bitmaps), Copyright 2022 Google Inc., licensed under the SIL Open Font
License, Version 1.1. The copyright and license strings are kept in the
font's name table.

Glyphs kept:

- U+1F7E2, U+1F534, U+1F535, U+26AA, U+2705, U+1F600: emoji the tests
  render.
- U+25B6, U+26A0, U+2714: UI symbols the font also maps.
- U+2764, U+260E: signs that are emoji only with U+FE0F after them.
- U+1F468, U+1F469, U+1F467, U+200D, U+1F1EF, U+1F1F5, U+1F44D, U+1F3FD,
  U+0031, U+20E3: the parts of four emoji sequences, and the one glyph the
  font draws each sequence with (glyph ids 2112, 1647, 2059 and 1478 of
  the source font): the family U+1F468 U+200D U+1F469 U+200D U+1F467, the
  flag U+1F1EF U+1F1F5, the thumb with a skin tone U+1F44D U+1F3FD and the
  keycap U+0031 U+20E3.

The substitution table is kept, cut down to the glyphs kept, so the font
joins those four sequences and no others.

Regenerate with fonttools:

    pyftsubset NotoColorEmoji.ttf \
        --unicodes=U+1F7E2,U+1F534,U+1F535,U+26AA,U+2705,U+25B6,U+25BC,U+26A0,U+2714,U+2192,U+2713,U+2764,U+260E,U+1F600,U+1F468,U+1F469,U+1F467,U+200D,U+1F1EF,U+1F1F5,U+1F44D,U+1F3FD,U+0031,U+20E3 \
        --gids=2112,1647,2059,1478 \
        --no-layout-closure --layout-features='*' --name-IDs='*' --notdef-outline \
        --output-file=color-emoji-subset.ttf

U+25BC, U+2192 and U+2713 are requested but absent from the source font.
