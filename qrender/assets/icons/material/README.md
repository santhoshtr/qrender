# Material Symbols

Icons for the composed items that Codex does not have. Use a Codex icon
when Codex has one.

- Source: https://github.com/google/material-design-icons, the files
  `symbols/web/<name>/materialsymbolsoutlined/<name>_wght600_20px.svg`.
- Style: Outlined, fill 0, weight 600, optical size 20. Weight 600 gives
  approximately the line width of the Codex icons.
- Licence: Apache License 2.0 (`LICENSE`).

Changes to each file: no `width` and `height`, and the `viewBox` is
`48 -912 864 864` (not `0 -960 960 960`). A Material Symbol has an empty
margin of 2px at 20px; a Codex icon has almost none. The smaller box
removes 1px on each side, so the two sets have approximately the same
size. No icon has a part in that 1px.

To add an icon: get the file as above, make the same changes, then add
its name to the table in `qrender/src/render/icons.rs`.
