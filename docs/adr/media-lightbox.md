# Media lightbox: a picture, a video, a 3D model or a map in a dialog

Status: done (2026-10-19), with the notes of §7.

## 1. The problem

The page showed each Commons file of the item in a small frame:

- A picture was a link to its Commons page. The reader left the page to
  see it larger.
- A video played in its grid cell. The cell was small, and the caption
  was on top of the video.
- A 3D model showed only the picture that Commons renders from the file.
  The reader could not turn the model.
- A map was a static picture of 800 × 500 pixels. The reader could not
  move it or zoom it.
- The author and the licence were behind an (i) button.

## 2. The decision

On click, a file of the item opens in a dialog: large, with its controls,
and with its attribution under it. The page stays where it is.

Scope: the files of the item (the `Media` block in every role: identity,
banner, feature, run, stack; also in a pictures block) and the maps (the
map block and the map of a coordinate pin). The pictures of item values
(`figure.html`: a gallery, the strip of a property, peers, works) show
another item; they do not change.

This is a change of layer 3 (templates, CSS, JavaScript) and of the
server (two routes for Leaflet). The composition, the view, the Markdown
and the fetcher do not change.

## 3. Design

### 3.1 One dialog for each figure, rendered by the server

The server writes a `<dialog>` after each figure, as for "View all". The
picture is an invoker button that opens it:

```html
<figure class="m k-photo" style="--a:1.5">
  <button class="zoom" type="button" command="show-modal" commandfor="d7"
          aria-label="View larger"><img src="…?width=960" …></button>
  <figcaption>…</figcaption>
  <span class="tools">…</span>
</figure>
<dialog id="d7" class="lightbox k-photo" closedby="any">…</dialog>
```

- The server renders the whole content. A script does not build the
  dialog.
- It works with no JavaScript: browsers have invoker commands, and
  `invokers.js` adds them to older browsers.
- The dialog is after the figure, not in it: in the figure, the styles of
  `.m figcaption` (absolute, white text) would reach it. A closed dialog
  is `display: none`, so it takes no grid or flex cell.
- The large picture is `loading="lazy"` and a video is
  `preload="none"`: a closed dialog loads nothing.
- The link to Commons moves from the picture into the dialog.

### 3.2 The dialog content

`parts/lightbox.html`, rendered by `Ui::lightbox`:

```
┌──────────────────────────────────────── × ┐
│ <property> (or <property>: <subject>)      │
├────────────────────────────────────────────┤
│      the file, as large as the window      │
├────────────────────────────────────────────┤
│ <qualifiers>                               │
│ <artist> · <licence>                       │
│ <file → Commons page> · <size> · ★ · ✎     │
└────────────────────────────────────────────┘
```

The CSS is in `base.css`, after `dialog`: `inline-size: min(1200px,
100vw - 2 × gutter)`, `max-block-size: 95vh`; the file has
`max-block-size: 70vh` and keeps its aspect ratio (`object-fit:
contain`). A drawing is on white; the others on the dark of `.m`.

### 3.3 Each kind

| Kind | On the page | In the dialog |
| --- | --- | --- |
| photo, drawing | the picture, a button ("View larger") | the picture, the full `srcset` |
| video | the poster with a ▶ mark and the size; no player in the cell ("Play") | the player; it starts on open and stops on close |
| 3D model | the Commons picture with a "3D" mark and the size ("View in 3D") | the model that the reader turns (3.4) |
| map | the static map, a button ("Explore the map") | the map that the reader moves and zooms (3.5) |
| audio | no change | no dialog |

The poster in the video dialog has the URL of the poster on the page
(960 px): the browser has it in its cache.

### 3.4 The 3D viewer

A port of the prototype's `prototype/v1/viewer3d.js` (WebGL2, no
library), not three.js: three.js with its STL loader is about 600 KB, the
port is about 7 KB, and Commons has one 3D format (STL).

`<qr-model>` in `assets/model.js`, a light-DOM custom element in the
dialog. It holds the Commons picture and a status line:

- When its dialog opens, it downloads the file and shows the progress on
  the picture. Then a canvas replaces the picture. The reader turns the
  model (drag; on a touch screen too, `touch-action: none`) and zooms it
  (wheel). It turns slowly until the first drag, and not with reduced
  motion.
- A close before the end stops the download (`AbortController`); the
  next open starts again. A model that is loaded stays.
- The model is drawn only while its dialog is open: the frame loop stops
  on close and starts again on open.
- The progress is the bytes received over `data-bytes`, the size that
  Commons gives. Not `Content-Length`: upload.wikimedia.org compresses an
  STL file (29 MB is 12.7 MB on the wire), and the reader gives the bytes
  after decompression.
- A file that is not binary STL (its size is not 84 + 50 × count, as an
  ASCII STL): the picture stays, with a message.
- With no WebGL2, the picture stays.
- The URL is `picture.url` (upload.wikimedia.org, which sends
  `Access-Control-Allow-Origin: *`).
- Colours: the model is grey on a transparent canvas; the stage gives
  the background.

### 3.5 The map

**Library.** Leaflet 1.9.4 (BSD-2-Clause), `leaflet.js` and
`leaflet.css` in `assets/vendor/leaflet/`, not changed. The design route
serves them (`/design/{version}/leaflet.js`, `leaflet.css`), as
`echarts.js`. Not the images of the package: the pin and the markers are
`divIcon`s with our classes.

**Tiles.** `https://maps.wikimedia.org/osm-intl/{z}/{x}/{y}{r}.png?lang=`
the page language: the place names are in the language of the page.
Attribution "© OpenStreetMap contributors", a link to the copyright page
of OpenStreetMap; no Leaflet prefix. (On a local server the tiles are
blank: they need a Wikimedia or Toolforge referrer.)

**Data.** The server gives the data; the script draws it. The dialog holds
`<qr-map data-map="…">` (Askama escapes the attribute), and in it the
static map of the page as the fallback (the same tile URL: the browser
has it). `render/map.rs` `live` makes the JSON:

```json
{"pin": [22.8, 83.0],
 "bounds": [[1.4112, 47.6223], [41.2486, 117.9348]],
 "markers": [{"at": [35.6745, 76.8445], "label": "coordinates of northernmost point"}],
 "outlines": [{"rings": [[[lat, lon], …]], "lines": [], "main": true}]}
```

- `bounds` is the box of the static map of the page (`Frame.bounds`):
  `render/map.rs` already chooses its view (the shape fits; else the
  extent of the item is a third of the width). Leaflet fits the box to
  its larger frame, so the dialog starts where the page map is.
- Places are latitude, longitude (as Leaflet takes them), rounded to 4
  decimals (about 11 m). Each shape is its closed rings (a polygon, with
  `evenodd` fill) and its open lines (no fill).
- A marker label is the property label in the page language.

**Script.** `assets/map.js`, the element `qr-map`. When its dialog opens
the first time, it loads `leaflet.css` and `leaflet.js` and waits for
both, then draws. It draws only in an open dialog: Leaflet must know the
size of its box. A later open calls `invalidateSize`.

**Look.** The classes of the static map are the classes of the Leaflet
layers (`.pin`, `.marker`, `path.main`, `path.other`, `.open`), so the CSS
in `map.css` is one for both. Leaflet writes colours and
`fill-opacity` as SVG attributes; the CSS overrides them.

**Under the map**, in its dialog: a legend of the markers (on the page a
marker has only a `title`) and the lines of the shapes (the same part as
under the page map, `parts/map_shapes.html`).

**The map of a coordinate pin** (📍 in `parts/part.html`) gets the same
dialog (`parts/map_dialog.html`), with a point frame (`point_frame`) and
no legend.

### 3.6 Scripts and assets

`lightbox.js` (the video), `model.js` and `map.js` are page scripts
(`PAGE_SCRIPTS`), not scripts of a primitive: a media block is also in a
pictures block, and a coordinate pin can be in any block. So the plan to
let a primitive have two scripts was dropped. `model.js` and `map.js` are
in one function scope each: the page scripts share one `<script>`
element. Leaflet is not in the page: `map.js` loads it from the design
route.

The `toggle` and `close` events of a dialog do not bubble: `lightbox.js`
hears them in the capture phase; the elements listen on their dialog.

Messages: `qrender-view-larger`, `qrender-play`, `qrender-view-model`,
`qrender-model-loading`, `qrender-model-unsupported`,
`qrender-explore-map`, `qrender-map-attribution`.

## 4. Commits

1. A picture opens in a dialog (with the part, the CSS and the messages).
2. A video plays in its dialog (`lightbox.js`).
3. A 3D model turns in its dialog (`model.js`).
4. A map in its dialog is moved and zoomed (Leaflet, `map.js`), also the
   map of a coordinate pin; this record.

## 5. Tests

- `pages.rs`: each `button.zoom` opens a `dialog.lightbox`; the
  `src` of a `qr-model` is on a Wikimedia host; the `data-map` of each
  `qr-map` parses, with a pin and a box whose south-west is below and
  left of its north-east. The checks before stay (dialog targets, unique
  ids, Wikimedia hosts, no link in a link).
- `render/map.rs`: the box of a frame is around its centre and is not
  wrapped across the antimeridian; a point frame has its point in the
  middle; a pole is at the edge of the map.
- `qrender-server`: the design route serves `page.css`, `echarts.js`,
  `leaflet.js` and `leaflet.css` with their types; another file is 404.
- `model.js` and `map.js` have no automatic test (the project has no JS
  tests). Checked in a browser: Earth (Q2: the 3D model, 29 MB, about 3 s;
  drag, wheel, close during the download; the coordinate pins, in
  Malayalam), the Moon (Q405), Nairobi (Q3870: the video plays on open
  and stops on close; nothing large loads with the page), India (Q668:
  two shapes, five markers, the legend, zoom).

## 6. Costs

- A page has a dialog for each file of the item: more HTML, no more
  requests until a dialog opens.
- The JSON of the outlines is in the page. India (two shapes of up to
  1,500 points): 81 KB → 98.5 KB gzip. Paris and Nairobi: about 1 KB.
  At 3 decimals (about 110 m) it would be about a quarter less.
- The page scripts are about 10 KB more on every page.

## 7. Notes

- Web Mercator ends at 85.05°: `mercator` clamps a latitude to it. A map
  of the North Pole shows the north edge of the map with the pin on it.
- No fixture has a coordinate pin, so the snapshots do not cover the
  pin's dialog. It was checked on Earth.
- The static map tile has no `lang`; the map in the dialog has. They can
  show place names in two languages.
- The marker is a diamond by its `::before`, not by a rotation of its
  box: Leaflet places a marker by its `transform`, and `rotate` applies
  before `transform`, so it turned the place of the marker too.
- Not done: a keyboard to turn a 3D model, pinch zoom of a 3D model, the
  Commons file description under a file (a fetcher change, per
  language).
- Found during the work, not changed:
  - the design URL (`design/0.2`) has not changed since the first commit,
    and the stylesheet is `immutable` for a week: a reader keeps an old
    stylesheet after a deploy;
  - `closedby="any"` is not in Safari: there, a click outside does not
    close a dialog (Esc and × do);
  - the dialog of a picture shows its "media legend" in every language;
    the page language alone would be a good caption (a localize change).
