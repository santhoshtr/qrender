"""The network primitive: the item values of one property, as pictures.

Two forms:
- a set: the values of one property. It shows its first pictures, in the
  order the composer gives (pictured values first, or by billing), in one
  row that scrolls, and all the values are in a dialog.
- a gallery (role "gallery"): the small values of different properties of
  one spread, one picture each.

localize: nothing of its own (see primitives/__init__.py).
"""


def build(c, pid, sts, shown, key):
    """key: the order of the values, from the order rules of the composer."""
    sts = sorted(sts, key=key)
    return {
        "primitive": "network",
        "pid": pid,
        "values": [dict(c.statement(s), shown=i < shown) for i, s in enumerate(sts)],
        "priority": c.priority(pid, sts),
    }


def gallery(singles):
    """The small values of a spread: pictured first, then by priority."""
    singles.sort(key=lambda v: (v.get("image") is None, -v["priority"]))
    return {
        "primitive": "network",
        "role": "gallery",
        "values": singles,
        "priority": max(v["priority"] for v in singles),
    }


def html(b, ui):
    if b.get("role") == "gallery":
        figures = "".join(ui.item_figure(v, v["property"]) for v in b["values"])
        return f'<div class="block set"><div class="run">{figures}</div></div>'
    shown = [v for v in b["values"] if v.get("shown", True)]
    rest = len(b["values"]) - len(shown)
    more = ""
    if rest:
        n = ui.dialog(ui.t(b["property"]), "".join(ui.value_line(v) for v in b["values"]))
        more = f'<button class="more" type="button" command="show-modal" commandfor="d{n}">+{rest}</button>'
    figures = "".join(ui.item_figure(v) for v in shown)
    return (
        f'<div class="block set"><h3>{ui.t(b["property"])} <span class="note">{len(b["values"])}</span></h3>'
        f"{ui.scroller(b['property']['text'], figures)}{more}</div>"
    )
