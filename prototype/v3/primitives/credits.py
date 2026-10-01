"""The credits primitive: the people who made a work, each person once.

The crew properties of a spread (layouts.toml role "crew": director,
screenwriter, producer, ...) are one block. A person who had more roles is
one entry with all of them: the director who also wrote and produced the
film. The entries keep the order of the properties: the director first.

localize: nothing of its own. Each person has "item" (a label and a page
image), "pids" (a label for each role) and "statement" with "pid" (an edit
link), and the generic walk of localize.py gives all of them.
"""

from values import entity_id


def build(c, pids, shown):
    people = {}
    for pid in pids:
        for s in c.take(c.free(pid)):
            key = entity_id(s["value"]) or s["value"]
            if key not in people:
                people[key] = dict(c.statement(s), pids=[])
            if pid not in people[key]["pids"]:
                people[key]["pids"].append(pid)
    if not people:
        return None
    values = list(people.values())
    return {
        "primitive": "credits",
        "pids": [p for p in pids if any(p in v["pids"] for v in values)],
        "values": [dict(v, shown=i < shown) for i, v in enumerate(values)],
        "priority": max(c.priority(p) for p in pids),
    }


def html(b, ui):
    def with_roles(v):
        # The roles of the person are the first part of its note.
        return dict(v, note=v["properties"] + (v.get("note") or []))

    values = [with_roles(v) for v in b["values"]]
    shown = [v for v in values if v.get("shown", True)]
    rest = len(values) - len(shown)
    more = ""
    if rest:
        n = ui.dialog(", ".join(ui.t(p) for p in b["properties"]), "".join(ui.value_line(v) for v in values))
        more = f'<button class="more" type="button" command="show-modal" commandfor="d{n}">+{rest}</button>'
    name = ", ".join(p["text"] for p in b["properties"])
    figures = "".join(ui.item_figure(v) for v in shown)
    return f'<div class="block set">{ui.scroller(name, figures)}{more}</div>'
