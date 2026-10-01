# Static prototypes

Design studies for the factoid page. The direction is design first: we make the
page we want, then we teach the code to make it.

The first generation, one build script for each item, is in `v1/`. The second
generation, one composer for any item, is in `v2/`. The third, in `v3/`, splits
the work into three layers (content, composition, design) with a thin
renderer, and shares no code with the other two.

## Rules

- **Provenance.** Every label, number, date, image, audio file and link on a page
  comes from the qjson data of the item. Section names, "See all" and the mock
  Wikipedia header are the only text that is not data.
- Wikimedia Codex design tokens, light theme.
- Static HTML. Content never needs JavaScript. Large value sets open in a
  `<dialog>` with invoker commands; a small inline script only helps browsers
  that do not support them yet.
- Media comes from Wikimedia Commons and maps.wikimedia.org.

## Pages

| Page | Item | Labels | Notes |
| --- | --- | --- | --- |
| `v1/q90.html` | Paris (Q90) | en | place, 435 statements |
| `v1/q3870.html` | Nairobi (Q3870) | gu | place, 58 statements |
| `v1/q8023.html` | Nelson Mandela (Q8023) | en | person, 190 statements, 89 of them awards |
| `v1/q8023-tcy.html` | Nelson Mandela (Q8023) | tcy | the same page, Tulu labels where they exist |
| `v1/q117085614.html` | One Battle After Another (Q117085614) | en | film, 239 statements |
| `v1/q117085614-ml.html` | One Battle After Another (Q117085614) | ml | the same page, no Malayalam article behind it |
| `v1/q41181.html` | Golden eagle (Q41181) | en | taxon, 232 statements |
| `v1/q41181-hi.html` | Golden eagle (Q41181) | hi | the same page, no Hindi article and no Hindi name |
| `v1/q405-ig.html` | Moon (Q405) | ig | astronomical object, 235 statements, a one-line Igbo article, a 3D model |

## Second generation: composed by rule

`v2/compose_v2.py` composes any item by rule, with no item-specific code. It
reads the qjson fixture, one side bundle from `v2/fetch_v2.py`, the judgment
in `v2/v2.toml` and the card groups of `qrender/groups.toml`. It uses the page
kit, the fixtures, the 3D viewer and the token stylesheet of `v1/`.

| Page | Item | Labels |
| --- | --- | --- |
| `v2/q405-ig-v2.html` | Moon (Q405) | ig |
| `v2/q3870-gu-v2.html` | Nairobi (Q3870) | gu |
| `v2/q41181-hi-v2.html` | Golden eagle (Q41181) | hi |

```bash
python3 prototype/v2/fetch_v2.py Q405 ig     # writes v2/data/q405-ig.json (network)
python3 prototype/v2/compose_v2.py Q405 ig   # writes v2/q405-ig-v2.html (offline)
```

## Third generation: three layers

```bash
python3 prototype/v3/content.py Q3870 en   # layer 1: data/content/q3870.json and data/text/q3870.en.json (network)
                                           # an existing content file is kept; add --refresh to fetch it again
python3 prototype/v3/compose.py Q3870      # layer 2: data/composition/q3870.json (no labels, any language)
python3 prototype/v3/localize.py Q3870 en  # localize: data/view/q3870.en.json (every string resolved)
python3 prototype/v3/render.py Q3870 en    # templates only: v3/q3870-en.html, with design/ as layer 3
```

The composition does not depend on the language: `v3/q3870-gu.html` is rendered
from the same composition file with the Gujarati text file.

The decisions of a facts spread are ordered rule tables at the top of
`compose.py`. Some primitives have one module each in `primitives/` (credits,
network, succession), with the composition and the template of the primitive
together.

Pages: Nairobi (Q3870) in English and Gujarati, Paris (Q90) in English and
Malayalam, Nelson Mandela (Q8023) in Tulu, One Battle After Another
(Q117085614) in English. The same programs make all of them.

## Build

```bash
python3 prototype/v1/build_q90.py                 # writes v1/q90.html
python3 prototype/v1/build_q3870.py               # writes v1/q3870.html
python3 prototype/v1/build_q8023.py               # writes v1/q8023.html
python3 prototype/v1/build_q8023.py tcy           # writes v1/q8023-tcy.html
python3 prototype/v1/build_q117085614.py          # writes v1/q117085614.html
python3 prototype/v1/build_q117085614.py ml       # writes v1/q117085614-ml.html
python3 prototype/v1/build_q41181.py              # writes v1/q41181.html
python3 prototype/v1/build_q41181.py hi           # writes v1/q41181-hi.html
python3 prototype/v1/build_q405.py                # writes v1/q405-ig.html
python3 -m http.server --directory prototype 8777 # then open /v1/q90.html
```

`factoid_kit.py` holds the parts that are the same on every page: data
loading, value formatting, the components (chip, picture tile, fact row,
stat tile, dialog, action menu) and the page frame. Each `build_<qid>.py`
holds only the composition — which sections the page has and what goes in
them — because that is the part we are still designing. It is not a
generalisation yet; it only stops us copying plumbing.

Each generator reads one WDQS result file, so every string on a page has a
source row. Paris reads the repository fixture
`qjson/tests/fixtures/Q90.sparql.json`; Nairobi reads
`v1/data/q3870-gu.sparql.json`, the same query asked with the label language
`gu, mul, en`.

Open the pages over HTTP: maps.wikimedia.org needs an HTTP referrer and gives
403 to a `file://` page.

`v1/data/*-boundary.geojson` is a city outline from
`maps.wikimedia.org/geoshape`, which builds it from the OpenStreetMap relation
that property P402 names.

## What the film page showed

- A film has no poster on Commons, because posters are not free. What it does
  have is a title treatment (P154) and a photograph of a cinema marquee
  (P18). The head uses the photograph as the plate and the logo as the mark.
- Scores are the thing a reader comes for. Four review scores, each with its
  method and how many ratings it averages, sit above everything else.
- Credits are faces. A person who directed, wrote and produced the film is one
  card with three roles, not three rows; the same grouping puts Warner Bros.
  on one card as production company and distributor. Six of the eight crew
  cards have no picture, so they ask for one.
- 23 jurisdictions classify this film, each through its own property, and most
  of their rating symbols are on Commons. A grid of badges says more than a
  list of codes.
- A release is not a date. It is a premiere, a festival, and then four waves
  of countries over three days.
- **Order is a choice, and the data barely supports it.** A cast strip of six
  out of nineteen has to pick six. Wikidata records no billing order, and
  SPARQL returns statements in hash order, so the first version showed six
  arbitrary actors and left the lead out. The count of references on a
  statement turns out to be the nearest signal the data offers: for this film
  the six best referenced (4, 4, 4, 3, 3, 2) are exactly the six the article
  names, and the rest all sit at 1. The page says out loud that this is the
  order it used.

  The qjson query does not fetch that count. Here it comes from a second
  query into `v1/data/q117085614-refs.json`. Any long value list — awards, cast,
  members — needs the same signal, so the pipeline will have to fetch it.

  Rejected: the number of sitelinks of the value item ranks actors too, but it
  measures how famous a person is in large-language Wikipedias, not what this
  film's data says.

- **The plot is missing and cannot be fixed here.** Wikidata holds statements,
  not prose, so the page says that in the place where a plot would be and
  offers to open the Wikipedia article. An archetype can have a hole in it
  that no amount of Wikidata will fill.

## What the taxon page showed

- **A banner crops, and a photograph must not be cropped.** The first version
  of this page put the eagle photograph in the wide band that Paris and
  Nairobi use, and the band showed the bird's legs. A page banner (P948) is
  drawn to be cropped; a plain image (P18) is a photograph of the subject and
  has whatever shape the photographer chose. So this hero shows the whole
  picture at its own aspect ratio, capped at 300px tall, and fills the band
  around it with a blurred copy of itself. Nothing is cut and the band still
  reads as a hero.

  The rule that follows: crop only images that a property says are banners.

- A taxon has no life story and no credits. It has a name with an author and
  a year, a rank in a tree, a conservation status, a range, a voice and a
  place in other people's heraldry. Almost none of the earlier sections
  carried over; the components did.
- The title is the scientific name, because for a taxon that is the
  language-independent `mul` label. On the Hindi page the heading therefore
  reads *Aquila chrysaetos*, tagged `mul`, and that is correct rather than a
  fallback failure.
- The conservation status carries its own picture: the IUCN scale with the
  right step highlighted. One image says what a word cannot.
- A range needs no ranking. The map is the range and the country list under
  it is complete, so there is nothing to cut and no order to justify. Sorting
  57 chips by label is safe exactly because none of them is hidden.
- **Two gaps in one page.** Wikidata has 110 sitelinks for this bird,
  including Malayalam, Tamil, Bengali and Urdu, but no Hindi article; and of
  39 common names in 36 languages not one is Hindi, although India is in the
  range. The page states both, in the place where each belongs, and offers
  *Add a name in Hindi* next to the second one. The invitation is computed
  from the data, not written into the page.

## What the Moon page showed

- **The article exists, and it is one line.** The Igbo article *Ọnwa* is 171
  bytes: an infobox template and "Ọnwa (ọdịmara: ☾)". Both the name and the
  symbol are already in Wikidata. The page shows that line first, because a
  person wrote it, and the English lead under it, marked as English.
- **Property labels are done; values are not.** Of the statements a reader
  reads, Igbo has a label for 47 of 51 properties but for only 4 of 28 value
  items. The page counts this and lists the 24 missing values. This is the
  bounded task of the strategy document: a list that a community can finish.
- A `mul` label ("CAPSTONE", "Montes Pyrenaeus") is a name that is the same
  in every language. It is not a fallback, and the page does not mark it or
  ask for it.
- Every fallback string has a `lang` attribute and a quiet dotted underline.
  On a page where most values fall back, a tag on each one is noise.
- **Section titles are Wikidata items**, resolved through the same label
  chain. Three of nine have an Igbo label. The others show English and are
  marked like any other fallback.
- The orbit is drawn to scale from periapsis and apoapsis. It is almost a
  circle with the Earth visibly off centre, and both facts are true.
- The coordinate is on the Moon's own globe. A tile from maps.wikimedia.org
  would be a map of the Earth, so the map is the Moon texture that the item
  names (P4640), with the point placed by longitude and latitude.
- **The 3D model is 26 MB.** The figure shows the picture that Commons
  renders from the STL file. A button loads the file only when the reader
  asks, and a small WebGL viewer with no library shows it. The caption says
  that the relief is 10 times exaggerated; that comes from the Commons file
  page.
- No recording says "Ọnwa" in Igbo. There are four, in Swedish, German,
  American English and Armenian, so the page asks for one.
- A timeline would be a half-empty showpiece: one date 4.5 billion years ago
  and three spacecraft. There is none.

## When the article does not exist

Wikidata has 53 sitelinks for this film and not one of them is an Indic
Wikipedia. The Malayalam page therefore shows a Malayalam structure — 162
property labels and 55 values come back in Malayalam, including the cast, the
review score sources and the award names — above an English lead paragraph
that states plainly that no Malayalam article exists, and offers Translate.
This is the case that matters most for these languages: the data is there, the
prose is not.

A tie in a ranked list must not break on the label. Sorting the remaining cast
alphabetically put a different actor first in Malayalam than in English,
because the two scripts sort differently. The tiebreak is now the item number,
which is arbitrary but identical in every language.

## Text that is not Wikidata

Two blocks on these pages come from elsewhere, and both say so where they sit:

- **The lead paragraph** of the Wikipedia article, under the title row of every
  page. It is always the English article, so on a page that is not in English
  the action on it is "Translate" instead of "Edit".
- **One Wikiquote quotation**, on the Mandela pages.

Nothing else on any page comes from outside the item's own data.

## The same page in two languages

`build_q8023.py` takes a language argument. The composition, the sections and
the order do not change; only the data file and the header words do. The Tulu
page is the honest picture of Tulu on Wikidata today: the title falls back to
the language-independent `mul` label, the description to English, and seven
property labels and four values come back in Tulu. The page marks each string
that did not arrive in the page language with its own language tag, so the
reader can see what is missing rather than being told nothing is.

## Sister projects

The Mandela page carries one quotation from Wikiquote. It is the only string
on any page that is not Wikidata, it says so on the page, and it links to the
Wikiquote article. It is there to show what pulling a sister project in could
look like. Three problems come with it and none is solved:

1. Choosing one quote out of many is editorial work we are not entitled to do.
2. Not every item has a Wikiquote page, so nothing general follows from one
   that does.
3. The quote is in English. A page in another language needs that language's
   Wikiquote, which may not hold the item at all.

## What the person page showed

- A person has no map, so the hero changes shape: a portrait beside the name,
  with a blurred copy of the same picture behind it, and the signature on a
  plate of its own.
- The spine of a person is one axis. Marriages, party membership, prison and
  office are four properties with dated spans; drawn on one 1918–2013 line
  they say more together than any of them says alone. 27 years of prison is a
  bar, not a sentence.
- One property can hold half the item. 89 of 190 statements are awards. The
  page shows six with pictures and sends all 89 to a table.
- Which award to feature is a real risk of editorialising. The page picks the
  statement that carries the most qualifiers, which is a property of the data.
  For Mandela that is the Nobel Peace Prize, with its rationale, its prize
  money and the person he shared it with.
- A count is not a headline. "occupation 7" tells a reader nothing;
  "lawyer, political prisoner, screenwriter, freedom fighter, autobiographer,
  political activist, politician" tells them who he was. The page head now
  carries a portrait at a quarter of the width and cards a reader actually
  asks for, not a strip of numbers.
- A value with no picture asks for one. An empty tile in a row of children or
  awards shows an "Add image" affordance instead of a grey plate.
- Place items reached by a person (place of birth, of death, of burial) carry
  no coordinates in this query, so a person page cannot draw them on a map
  without a second query.

## What the second page showed

- Nairobi is not Paris with other values in it. With 58 statements, a page of
  nine full-width sections would be mostly empty, so the sections are fewer
  and sit two to a row.
- Three census counts, years apart, are bars from zero, not a line. A line
  says "measured over time"; these are three separate counts.
- The map frame comes from the outline, not from a fixed zoom.
- Nairobi has a video (P10) and Paris does not. A value type that appears on
  one item and not another is normal, and the page has to be able to grow a
  place for it.
- **Open question.** Section names ("Geography", "Government") are ours, not
  Wikidata's, so on the Gujarati page they stay in English. They need a
  message catalogue of their own. We will not invent translations.

## Section colours

The colours carry no meaning. Sections and key-fact tiles take them from the
Codex `option-*` family in page order, so the page looks rich and stays easy to
scan. Red and maroon stay free, because they read as destructive.

## Page shape

Banner hero, key-fact tiles, then sections: Names, Symbols, Geography,
Demographics, Government, History, Connections, Views and sound, and
On Wikimedia projects.

A picture tile is one full-bleed image with the name and the note on top of it,
over a gradient. Each of the two lines keeps to one line and ends in an
ellipsis; the full value stays in the `title` attribute.

Every statement carries its own three-dot button — a fact row, a chip, a picture
tile, a key-fact tile, a timeline row, the map, and the heading of a property
that a chart stands for. A card as a whole has none: you edit the mayor, not
the government of Paris. The button stays invisible until the value is hovered
or the button takes focus. All of them open the same popover, which the browser
tethers to the button that opened it (CSS anchor positioning, implicit anchor).
The menu holds one item, "Edit", to show the idea.

Large sets show one row and send the rest to a dialog: twin cities, mayors,
owned buildings, views, demonyms, sources and the 262 external identifiers.
