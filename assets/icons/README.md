# The app's icon

A serif capital M on a tile, in two forms. Both are original work for
md-viewer and are under the repository's license.

| File | What it is |
| --- | --- |
| `sepia.svg`, `sepia-512.png` | A dark M on a warm paper tile: the icon while Paper or Sepia is on |
| `reversed.svg`, `reversed-512.png` | The same M in paper on a near-black tile: the icon while Dusk or Night is on |

The SVG files are the sources, on a 1024 px artboard. The tile carries its
own margin, rounded corners and shadow, because macOS does not cut an icon
to shape when an app sets it while it runs.

Each PNG is its SVG drawn at 512 px on a transparent page by headless
Chromium: Playwright's `page.screenshot({ omitBackground: true })` of a
512 px viewport that holds the SVG alone. The PNG files are compiled into
the binary (`src/app_icon/`), and `make install` copies the sepia one for
the desktop's launcher.
