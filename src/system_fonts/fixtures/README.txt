script-samples.otf
==================

Test fixture for the font tests under src/system_fonts/. It stands in for the
script fallback fonts of a multilingual desktop, so those tests run the same
on a machine with no such fonts installed.

It is a subset of GNU Unifont 15.1.01, Copyright 1998-2023 Roman Czyborra,
Paul Hardy, Qianqian Fang, Andrew Miller, Johnnie Weaver, David Corbett,
Nils Moskopp, Rebecca Bettencourt, Minseo Lee, Ho-Seok Ee, et al. Unifont is
dual-licensed: SIL Open Font License version 1.1, and GNU GPL version 2 or
later with the GNU Font Embedding Exception. This repository takes it under
the SIL Open Font License. The copyright and license strings are kept in the
font's name table.

Glyphs kept: exactly the coverage samples of SCRIPT_FALLBACKS in
src/system_fonts/install/mod.rs (Han, Hiragana, Katakana, Hangul, Arabic,
Hebrew, Devanagari, Thai). It has no Latin letters on purpose: the tests
also use it as a family that cannot serve as a body font.

Regenerate with fonttools:

    pyftsubset unifont.otf \
        --text="中文测试繁體かなカナ한글ابאבनमस्तेहिन्दीสวัสดีภาษาไทย" \
        --no-layout-closure --layout-features= --name-IDs='*' --notdef-outline \
        --output-file=script-samples.otf

The Latin faces the same tests register come from egui's bundled fonts
(Ubuntu Light, Hack, Noto Emoji), read out of FontDefinitions::default().
