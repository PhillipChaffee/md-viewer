#!/usr/bin/env bash
#
# Checks that the website states what the code says. docs/index.html and
# docs/404.html are written by hand, and their facts (the version, the
# shortcuts, the theme colors, ...) are copied from the code. This reads each
# fact from the file that decides it and fails where a page says otherwise.
#
# A fact on a page is an element with a data-fact attribute. Its value is the
# element's text; an element with no text gives its `content` attribute (a
# <meta>) or the one custom property in its style (a color swatch). Every
# marked element must hold the value the code gives its name, every name must
# be marked somewhere, and a name the script does not know fails.
#
# Four scans need no markup: GitHub links point into the repository, the
# Pages address matches it, links into the README name a heading it has, and
# "four themes" and "three line spacings" count what the code has.
#
# Usage: scripts/check-site-facts.sh
# Requires: bash 4, python3 (standard library only).

set -u -o pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root" || exit 1

pages=(docs/index.html docs/404.html)
failures=0
declare -A expected origin unreadable

fail() {
	printf 'error: %s\n' "$1" >&2
	shift
	printf '  %s\n' "$@" >&2
	failures=$((failures + 1))
}

# fact NAME VALUE FILE PATTERN: the code gives NAME the value VALUE, at the
# first line of FILE that matches the extended regex PATTERN. An empty VALUE
# means the pattern below it found nothing, which is itself a failure.
fact() {
	local line
	line=$(grep -nE -m1 -- "$4" "$3" | cut -d: -f1)
	if [[ -z "$2" ]]; then
		fail "could not read fact \"$1\" from $3" "the pattern in scripts/check-site-facts.sh no longer matches: $4"
		unreadable[$1]=1
		return
	fi
	expected[$1]=$2
	origin[$1]="$3:${line:-?}"
}

# The value of KEY = "..." in the [package] table of Cargo.toml.
package_field() {
	sed -n '/^\[package\]$/,/^\[/{s/^'"$1"' = "\([^"]*\)"$/\1/p}' Cargo.toml
}

number_word() {
	local words=(zero one two three four five six seven eight nine ten)
	printf '%s' "${words[$1]:-$1}"
}

# Joins its arguments as a sentence lists them: "a", "a and b", "a, b and c".
join_list() {
	local out=$1
	shift
	while (($# > 1)); do
		out+=", $1"
		shift
	done
	(($# == 1)) && out+=" and $1"
	printf '%s' "$out"
}

# --- Cargo.toml -------------------------------------------------------------

version=$(package_field version)
fact version "$version" Cargo.toml '^version = '
fact licence "$(package_field license)" Cargo.toml '^license = '
repository=$(package_field repository)
fact repository "$repository" Cargo.toml '^repository = '
# The binary is named after the package unless a [[bin]] table renames it.
binary=$(sed -n '/^\[\[bin\]\]$/,/^\[/{s/^name = "\([^"]*\)"$/\1/p}' Cargo.toml | head -n 1)
binary=${binary:-$(package_field name)}
fact binary "$binary" Cargo.toml '^name = '
# `git clone` makes a folder named after the repository, the URL's last part.
fact clone-dir "${repository##*/}" Cargo.toml '^repository = '

# --- Makefile and rust-toolchain.toml ----------------------------------------

grep -q '^install:' Makefile && fact make-install "make install" Makefile '^install:'
# The folder the install recipe copies the release binary into.
install_dir=$(sed -n '/^install:/,/^$/{s|^\tcp target/release/'"$binary"' \(.*[^/]\)/*$|\1|p}' Makefile)
fact install-dir "$install_dir" Makefile "^	cp target/release/$binary "
[[ -f rust-toolchain.toml ]] && fact toolchain-file rust-toolchain.toml rust-toolchain.toml '^channel = '

# --- src/theme: the four themes ----------------------------------------------

theme_rs=src/theme/mod.rs
# ThemeId::ALL is the order of the themes, and the order Ctrl+D steps in.
read -ra theme_ids <<<"$(sed -n 's/^ *pub(crate) const ALL: \[Self; [0-9]*\] = \[\(.*\)\];$/\1/p' "$theme_rs" | sed 's/Self:://g; s/,//g')"
theme_labels=()
for id in "${theme_ids[@]}"; do
	# ThemeId::label: `Self::Paper => "Paper",`.
	label=$(sed -n 's/^ *Self::'"$id"' => "\([^"]*\)",$/\1/p' "$theme_rs")
	theme_labels+=("$label")
	# ThemeId::palette: `Self::Paper => &PAPER,`, then that palette's page.
	palette=$(sed -n 's/^ *Self::'"$id"' => &\([A-Z_]*\),$/\1/p' "$theme_rs")
	page_raw=$(sed -n '/^const '"$palette"': Palette = Palette {$/,/^};$/{s/^ *page: rgb(0x\([0-9A-Fa-f_]*\)),$/\1/p}' "$theme_rs")
	page_hex=${page_raw//_/}
	# Keyed by the variant, so renaming a theme fails on its name alone.
	fact "colour:$id" "${page_hex:+#$page_hex}" "$theme_rs" "^ *page: rgb\(0x${page_raw:-?}\)"
done
fact theme-names "$(join_list "${theme_labels[@]}")" "$theme_rs" 'const ALL: \[Self; '
# The swatch row lists the names one per item, read with a space between.
fact theme-swatches "${theme_labels[*]}" "$theme_rs" 'const ALL: \[Self; '
theme_count=${#theme_ids[@]}

# The page's own CSS is "ink on the app's Paper palette": each token below is
# one field of PAPER, read where the page defines it.
declare -A css_fields=([paper]=page [ink]=text [head]=heading [muted]=muted [rule]=rule [link]=link [code]=inline_code)
declare -A css_hex
for token in "${!css_fields[@]}"; do
	css_hex[$token]=$(sed -n '/^const PAPER: Palette = Palette {$/,/^};$/{s/^ *'"${css_fields[$token]}"': rgb(0x\([0-9A-Fa-f_]*\)),$/\1/p}' "$theme_rs" | tr -d _)
done

# The contrast the theme tests hold body text and headings to: `ratio >= 7.0`.
contrast=$(sed -n 's/^ *ratio >= \([0-9.]*\),\{0,1\}$/\1/p' src/theme/tests.rs | head -n 1)
contrast=${contrast%.0}
fact body-contrast "${contrast:+$contrast:1}" src/theme/tests.rs '^ *ratio >= '

# --- src/layout: line spacing ------------------------------------------------

spacing_count=$(sed -n 's/^ *pub(crate) const ALL: \[Self; \([0-9]*\)\] = \[Self::Compact.*/\1/p' src/layout/mod.rs)
[[ -n "$spacing_count" ]] || fail "could not read LineSpacing::ALL from src/layout/mod.rs"

# --- src/app/shortcuts: the keys ---------------------------------------------

# The hint the app shows off the Mac for one row of BINDINGS, as
# src/app/shortcuts/hints writes it: "Ctrl+Shift+E".
hint() {
	local chord=$1 key=$2 mods
	case "$chord" in
	Cmd | CmdNoShift | CtrlNoShift) mods="Ctrl+" ;;
	CmdShift | CtrlShift) mods="Ctrl+Shift+" ;;
	CmdAlt) mods="Ctrl+Alt+" ;;
	AltNoCtrl) mods="Alt+" ;;
	Shift) mods="Shift+" ;;
	MacCmd) return ;; # a Mac's own key, which the page does not show
	*) mods="" ;;
	esac
	case "$key" in
	Num[0-9]) key=${key#Num} ;;
	Plus) key="+" ;;
	Minus) key="-" ;;
	Equals) key="=" ;;
	OpenBracket) key="[" ;;
	CloseBracket) key="]" ;;
	esac
	printf '%s%s\n' "$mods" "$key"
}

shortcuts_rs=src/app/shortcuts/mod.rs
declare -A keys_of
# A row of the table: `    CmdNoShift + E in Document => AppAction::ToggleEditMode,`.
while read -r chord key action; do
	keys_of[$action]+="$(hint "$chord" "$key")|"
done < <(sed -n 's/^ *\([A-Za-z]*\) + \([A-Za-z0-9]*\) in [A-Za-z]* => AppAction::\([A-Za-z]*\).*/\1 \2 \3/p' "$shortcuts_rs")
for action in ToggleEditMode Save NextTheme; do
	fact "shortcut:$action" "${keys_of[$action]:-}" "$shortcuts_rs" "=> AppAction::${action}[,(]"
done

# --- the platforms CI type-checks --------------------------------------------

cross=.github/workflows/cross-platform.yml
platforms=()
for os in $(sed -n 's/^ *os: \[\(.*\)\]$/\1/p' "$cross" | tr -d ,); do
	case "$os" in
	macos-*) platforms+=(macOS) ;;
	windows-*) platforms+=(Windows) ;;
	ubuntu-*) platforms+=(Linux) ;;
	*) platforms+=("$os") ;;
	esac
done
fact type-checked-platforms "$(join_list "${platforms[@]}")" "$cross" '^ *os: \['

# --- the bundled fonts --------------------------------------------------------

bundled_rs=src/system_fonts/bundled/mod.rs
# The families compiled in, by file name: bundled_face!("SourceSerif4-Bold.ttf").
mapfile -t faces < <(sed -n 's/.*bundled_face!("\([A-Za-z0-9]*\)-[^"]*").*/\1/p' "$bundled_rs" | awk '!seen[$0]++')
# The page names a family with spaces ("Source Serif 4"); compared without.
fact bundled-faces "$(join_list "${faces[@]}")" "$bundled_rs" 'bundled_face!\("'
font_licence="SIL Open Font License"
for prefix in "${faces[@]}"; do
	grep -qi "$font_licence" "assets/fonts/$prefix-LICENSE.txt" 2>/dev/null || font_licence=""
done
fact font-licence "$font_licence" "assets/fonts/${faces[0]:-?}-LICENSE.txt" 'SIL Open Font License'

# --- README.md: where the page sends readers ----------------------------------

# The package lists are under ### System packages, one bold label each:
# **Debian and Ubuntu**, **Arch Linux**.
systems=()
while IFS= read -r label; do
	systems+=("${label% and *}")
	[[ "$label" == *" and "* ]] && systems+=("${label##* and }")
done < <(sed -n '/^### System packages$/,/^### [^S]/{s/^\*\*\(.*\)\*\*$/\1/p}' README.md)
fact package-systems "$(join_list "${systems[@]}")" README.md '^### System packages$'

# GitHub's anchor for each heading outside a code fence.
readme_anchors=$(awk '/^```/ { fenced = !fenced; next } !fenced && /^#+ / { sub(/^#+ /, ""); print }' README.md |
	tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9 _-]//g; s/ /-/g')

# --- the pages ------------------------------------------------------------------

# Each marked element as `file<TAB>line<TAB>name<TAB>value`.
marked=$(
	python3 - "${pages[@]}" <<'PY'
import re
import sys
from html.parser import HTMLParser

VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}


class Facts(HTMLParser):
    def __init__(self, path):
        super().__init__(convert_charrefs=True)
        self.path = path
        self.open = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        fact = None
        if "data-fact" in attrs:
            fact = {"name": attrs["data-fact"], "line": self.getpos()[0], "attrs": attrs, "text": []}
        if tag in VOID:
            if fact:
                self.emit(fact)
            return
        self.open.append((tag, fact))

    def handle_endtag(self, tag):
        while self.open:
            name, fact = self.open.pop()
            if fact:
                self.emit(fact)
            if name == tag:
                break

    def handle_data(self, data):
        for _, fact in self.open:
            if fact:
                fact["text"].append(data)

    def emit(self, fact):
        value = " ".join(" ".join(fact["text"]).split())
        if not value:
            value = fact["attrs"].get("content") or ""
        if not value:
            style = re.fullmatch(r"\s*--[\w-]+\s*:\s*([^;]+?)\s*;?\s*", fact["attrs"].get("style") or "")
            value = style.group(1) if style else ""
        print(f"{self.path}\t{fact['line']}\t{fact['name']}\t{value}")


for path in sys.argv[1:]:
    with open(path, encoding="utf-8") as page:
        parser = Facts(path)
        parser.feed(page.read())
        parser.close()
PY
) || {
	fail "could not read the data-fact elements of ${pages[*]}"
	marked=""
}

declare -A seen
while IFS=$'\t' read -r file line name value; do
	[[ -n "$file" ]] || continue
	seen[$name]=1
	[[ -n "${unreadable[$name]+set}" ]] && continue # already reported
	if [[ -z "${expected[$name]+set}" ]]; then
		fail "$file:$line marks an unknown fact \"$name\"" "scripts/check-site-facts.sh reads no such fact from the code; fix the name or add it there"
		continue
	fi
	want=${expected[$name]}
	ok=false
	case "$name" in
	shortcut:*) [[ "|$want" == *"|$value|"* ]] && ok=true ;;
	bundled-faces) [[ "${value// /}" == "${want// /}" ]] && ok=true ;;
	colour:*) [[ "${value^^}" == "${want^^}" ]] && ok=true ;;
	*) [[ "$value" == "$want" ]] && ok=true ;;
	esac
	if [[ "$ok" == false ]]; then
		fail "site fact \"$name\" disagrees with the code" \
			"code: ${want%|} (${origin[$name]})" \
			"page: $value ($file:$line)"
	fi
done <<<"$marked"

for name in "${!expected[@]}"; do
	[[ "$name" == repository ]] && continue # checked by the link scan below
	[[ -n "${expected[$name]}" ]] || continue
	if [[ -z "${seen[$name]+set}" ]]; then
		fail "no element on ${pages[*]} carries data-fact=\"$name\"" \
			"code: ${expected[$name]%|} (${origin[$name]})" \
			"mark the element that states it, or drop the fact from scripts/check-site-facts.sh"
	fi
done

owner_repo=${repository#https://github.com/}
pages_url="https://${owner_repo%%/*}.github.io/${owner_repo#*/}/"
pages_url=${pages_url,,}
for file in "${pages[@]}"; do
	# Every GitHub address on the page is the repository or a page inside it.
	while IFS=: read -r line url; do
		case "$url" in
		"$repository" | "$repository"/* | "$repository"#*) ;;
		*) fail "$file:$line links to $url" "code: the repository is $repository (${origin[repository]})" ;;
		esac
		if [[ "$url" == "$repository#"* ]]; then
			anchor=${url#*#}
			grep -qxF -- "$anchor" <<<"$readme_anchors" ||
				fail "$file:$line links to #$anchor, which README.md has no heading for" "code: README.md headings give: $(tr '\n' ' ' <<<"$readme_anchors")"
		fi
	done < <(grep -noE 'https://github\.com/[^"<> ]*' "$file")
	# The site's own address, in the social-card tags: GitHub Pages of the repository.
	while IFS=: read -r line url; do
		[[ "${url,,}" == "$pages_url"* ]] ||
			fail "$file:$line gives the site's address as $url" "code: GitHub Pages serves $repository at $pages_url (${origin[repository]})"
	done < <(grep -noE 'https://[^"/<> ]*\.github\.io/[^"<> ]*' "$file")
	# "Four themes", "three line spacings": a count of what the code has.
	while IFS=: read -r line phrase; do
		read -r word noun <<<"${phrase,,}"
		case "$noun" in
		theme*) want=$(number_word "$theme_count") where="$theme_rs ThemeId::ALL" ;;
		*) want=$(number_word "$spacing_count") where="src/layout/mod.rs LineSpacing::ALL" ;;
		esac
		[[ "$word" == "$want" ]] ||
			fail "site count \"$phrase\" disagrees with the code" "code: $want ($where)" "page: $word ($file:$line)"
	done < <(grep -noiE '\b(one|two|three|four|five|six|seven|eight|nine|ten) (themes?|line spacings?)\b' "$file")
	# The page's CSS tokens that name a color of the Paper palette.
	for token in "${!css_hex[@]}"; do
		while IFS=: read -r line decl; do
			value=${decl#*:}
			[[ "${value^^}" == "#${css_hex[$token]^^}" ]] ||
				fail "CSS token --$token disagrees with the Paper palette" \
					"code: #${css_hex[$token]} (PAPER.${css_fields[$token]} in $theme_rs)" \
					"page: $value ($file:$line)"
		done < <(grep -noE -- "--$token:#[0-9A-Fa-f]{6}" "$file")
	done
done

if ((failures > 0)); then
	printf '%d site fact(s) disagree with the code. Fix the page, or the code if the page is right.\n' "$failures" >&2
	exit 1
fi
printf 'site facts match the code: %d facts, version %s\n' "${#expected[@]}" "$version"
