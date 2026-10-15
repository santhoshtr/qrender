# Web presence: the item's sites and accounts as cards with logos

Status: approved (2026-10-15).

## 1. The problem

The web properties (official website, X, Facebook, subreddit, hashtag,
open data portal) are rows in the evidence spread. An account is its bare
name ("india"), not a link: the page does not know the formatter URLs.
A reader looks for these links; they are not curation data.

## 2. Decisions

- **A spread, `web`** (kind `web`, concept Q22909728 "web presence"), in
  every class layout, before "more". It takes the properties of the `web`
  role. The evidence spread has no web slot.
- **A primitive, `links`:** a card for each value: the logo of its
  platform, the label of the property, and the value as a link.
  - The address: a URL value is the address; an identifier or a string
    goes into the formatter URL (P1630) of its property.
  - The logo: the logo (P154) of the platform item. The configuration
    names the platform of each property (`[web.platforms]`), because the
    properties do not name it reliably (X username names "X account").
  - Without a platform: an official website has a globe icon, a hashtag a
    "#"; an item value (an open data portal) has its own picture.
  - Followers (P8687) go on the card of the account that their qualifier
    names (subreddit "india"); the latest count. Followers of no card stay
    rows.
- **The fetcher** gets the formatter URL of each web property of the item
  (one query) and the logos of their platforms (the logo query).
  `FETCH_VERSION` 6.
- **The web role:** Quora topic ID, TikTok username, Mastodon address and
  YouTube handle join it. IATA airport code leaves it: it is a code.

## 3. Changes

- `content`: `Settings.web`, `Settings.platforms`; `Content.formatters`;
  `sparql::formatters`; the platform items in the logo query.
- `config`: `[web] platforms`.
- `vocab`: `SpreadKind::Web`, `Primitive::Links`.
- `primitives/links.rs`, its template and CSS; `compose/spreads/web.rs`.
- Tests: the cards (address, logo, followers); snapshots.
