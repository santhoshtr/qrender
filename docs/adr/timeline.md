# Timeline: events that are items of their own, in the history spread

Status: done (2026-10-14).

## Decisions

- **A role, `timeline` (P793 significant event).** The history spread
  makes a vertical timeline of its values: one full row for each event,
  with its date, its picture (the value image, else a page image), its
  label and its description. The oldest first; for the same first year,
  the event that ends first; the undated last.
- **The date** is the statement's (point in time, else start and end
  time). Else the event item's own: start and end time, else point in
  time (a war's point in time is often only one of its ends). The fetcher
  reads the dates of the item values of timeline properties
  (`Content.event_dates`, one query; `FETCH_VERSION` 2). The USA has 41
  events and no date on any statement: all its dates are the items'.
- **No time axis.** The horizontal axis (events and history spans of
  several properties on one line) was crowded for the USA: 41 events and
  their labels. It is removed, with its roles (`event`, `history_span`)
  and its threshold. The history spread is the timeline of the
  significant events only; without one, it stays for its topic or its
  prose. Inception, replaces and former countries go to the spreads that
  list them, or to "more".
- **All rows show**, no "+N": the timeline is the content of the
  history spread.
- **A facts spread with `pictured`** does not take a timeline property:
  the relations spread took the 41 events of the USA as pictures.
- The date qualifiers are not repeated in the note of an event: they are
  its date.
