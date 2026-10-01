"""Primitives: one module for each kind of block, with all its behavior.

Each module has the faces of its primitive, in the order of the pipeline:
- build(c, ...): the composition block, from the content, with no labels.
  c is the Composer: it claims statements and gives the statement form.
- localize: the generic walk of localize.py gives each item and property
  of a block its label, by the shape of the data. A primitive has a
  localize function only when it needs more than that.
- html(b, ui): the template, from the localized block. ui is the module of
  template helpers (render.py).
"""
