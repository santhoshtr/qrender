"""The succession primitive: an office and its holders.

The office is the title, the holder now is the lead picture, and the
holders before are newest first, a few shown and all of them in a dialog.
The role table (layouts.toml [roles.office]) gives the office property of
each holder property: P6 (head of government) -> P1313 (office held by
head of government).

localize: nothing of its own (see primitives/__init__.py).
"""

from values import qual_year


def build(c, holder_pid, office_pid, shown):
    holders = c.current(c.free(holder_pid))
    if not holders:
        return None
    c.take(holders)
    office = c.current(c.free(office_pid))
    office = c.take(office[:1])[0] if office else None
    now, before = holders[0], holders[1:]
    before.sort(
        key=lambda s: -(qual_year(s, "P580") or qual_year(s, "P582") or -(10**6))
    )
    return {
        "primitive": "succession",
        "pid": holder_pid,
        "office": c.statement(office) if office else None,
        "holder": c.statement(now),
        "before": [dict(c.statement(s), shown=i < shown) for i, s in enumerate(before)],
        "priority": c.priority(holder_pid, holders),
    }


def html(b, ui):
    office = ui.part(b["office"]["display"]) if b.get("office") else ui.t(b["property"])
    before = ui.value_lines(b["before"], office) if b["before"] else ""
    return (
        f'<div class="block succession"><h3>{office}</h3>{ui.item_figure(b["holder"], b["property"])}'
        + (f'<div class="before">{before}</div>' if before else "")
        + "</div>"
    )
