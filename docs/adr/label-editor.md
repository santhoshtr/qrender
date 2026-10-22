# The label editor: a fallback label in a dialog

Status: done (2026-10-22).

## 1. The problem

A label that is not in the page language (a fallback, from the chain
`{lang}, mul, en`) has a dotted underline and a "+" badge, on hover or
focus. The badge linked to `Special:SetLabel` on Wikidata, so the reader
left the page to add the label. The prototype must show the edit in the
page, as the translation of a section does
(`fallback-article-translation.md`).

## 2. Decisions

- **The action** is the Codex edit icon, not "+". It shows on hover, on
  keyboard focus, and on a click of a label that is not a link. A click
  on a label that is a link opens the link. Its `href` stays
  `Special:SetLabel/{id}/{language}`: without the script, it is the
  real action.
- **The dialog**, one for the page: every edit of the page is in a
  dialog. It shows the label in its language, and a field in the page
  language that the machine translation (MinT, by cxserver, as
  `translate.js`) fills first. The reader corrects it. "Save" and
  "Close" (and ×, Esc).
- **Save mimics a published edit.** Every place of the page with the
  same label (the same `href` of the action) shows the new label at
  once, in the page language, with a solid underline in the colour of
  success and the note "Saved on this page only" as its title. Nothing
  goes to a server; a reload shows the fallback again. The action opens
  the dialog again with the source label and the saved text, with no
  request.
- **Only fallback labels.** Not a description, and not a label in the
  page language.
- **No edit mode of the page.** A global switch that makes every label
  and value editable was considered, and so was a double click. A double
  click is hidden, it selects a word, a link opens on the first click,
  and touch screens and keyboards do not have it. An edit mode goes
  against the idea of this page: an edit is of one part, in its place,
  not "edit the article". The edit of a label that is in the page
  language is a question for later.

## 3. Design

- `macros.html` `add_label`: the edit icon in the action.
- `parts/label_editor.html`, rendered by `Ui::label_editor` in
  `page.html` when the view has a fallback label with an action
  (`Ui::has_label_actions`). The server writes every word of it in the
  page language: the title "Add the label in ગુજરાતી"
  (`qrender-label-title`), the note of the field, the failure
  (`qrender-label-failed`), the buttons, the saved note.
- `assets/label.js`, a page script (a fallback label can be in any
  block). Light DOM, no custom element: the labels are many small spans.
  - A click on an action opens the dialog (`preventDefault`).
  - The request to MinT sends the label as the text of a `<p>`; the
    script reads only the text of the answer, parsed in a `template`,
    which runs and loads nothing.
  - The language of the source label is named by `Intl.DisplayNames`,
    as `intl.js` names languages.
  - Save, Close; the request still running stops when the dialog closes
    (`AbortController`).
- CSS: the dialog (narrower than the translator), the mark of a saved
  label. The action bar of a dialog (the status, the buttons) is shared
  by the translator and the label editor.

## 4. Notes

- The action linked to `Special:SetLabel/{language}/{id}`. Wikidata
  reads the first part of the subpage as the id ("The ID "hi" is
  unknown"); the order is `{id}/{language}`, and the form then opens
  with both set. Fixed before this step (fcbffa1).
- The edit icon replaced "✎" in the edit action of a statement before
  this step (7c2696a). An inline icon loses the white space of its file:
  the edit icon is on a page a thousand times (Q30: 1010).
- Checked in a browser on `gu/Q3870`, a page from the fixtures served
  locally (Toolforge was down): MinT fills the field in about 2 s ("big
  city" → "મોટું શહેર"); Save changes the 3 places of the label; the
  action opens the saved text again with no request; a click on a label
  that is not a link shows its action, a click elsewhere hides it.
- `.from` is already the "W" mark of a page image: the language of the
  source label is `.source-lang`.
- The script repeats the request to MinT of `translate.js` (15 lines):
  each page script is one concern, and a shared global is more than the
  repeat.
- Known, not of this step: the action of a label at the end of a line
  goes out of the page by 9px, and so does a picture of a picture run (a
  horizontal scroll bar). Both were there before.
