#!/usr/bin/env python3
"""Layer 2, composition: the editorial decisions for one item.

Reads only the content layer (data/content/<qid>.json) and layouts.toml. It
never reads labels or any text, so the composition is the same for every
language. It writes data/composition/<qid>.json.

The class layout of the item (layouts.toml) says which spreads the page has,
in which order, and which properties each spread shows. The composer decides
the rest by rule:
- the form of each property, from the shape of its data and its role: a
  picture, a map with markers, an office with its holders, a set of faces, a
  list, a series, a row of the fact sheet;
- which value leads (the current one), which values are history (the ones
  that ended), which are never pictured (classes);
- the template of each spread, from whether its first block is a picture;
- what is left: collapsed at the end, never lost.

It holds values as identifiers and numbers, never as labels, and layout only
as template and slot names and intrinsic facts (aspect ratio, fraction of a
whole, position on the item's own time span).

Usage: python3 prototype/v3/compose.py Q3870
"""

import json
import math
import re
import sys
import tomllib
from pathlib import Path

from primitives import credits, network, succession
from values import entity_id, qual_year, qualifier_of, value_of, year_of

HERE = Path(__file__).resolve().parent
CONFIG = tomllib.loads((HERE / "layouts.toml").read_text())
ROLES = CONFIG["roles"]
TH = CONFIG["thresholds"]
SCHEMA = "composition/0.2"
DESIGN = "design/0.2"

AUDIO = (".ogg", ".oga", ".wav", ".mp3", ".flac", ".opus")
VIDEO = (".webm", ".ogv", ".mp4")


# ------------------------------------------------------------------ rules
# The decisions of a facts spread are data: ordered rules, and the first
# rule whose conditions all hold wins. A condition names a predicate and the
# value it must have. The predicates are computed in the Composer (the
# vocabulary); the rules only combine them.

# Before any statement of a property is claimed.
PROPERTY_RULES = [
    {"id": "office-title", "when": {"office_title": True}, "make": "skip"},
    {"id": "extreme-point", "when": {"role": "extreme_point"}, "make": "skip"},
    {"id": "office", "when": {"role": "office"}, "make": "succession"},
    {"id": "crew", "when": {"role": "crew"}, "make": "club"},
    {"id": "values", "when": {}, "make": "values"},
]

# Values of an item property that ended are history: a group of their own.
SPLIT_RULES = [
    {
        "id": "former-values",
        "when": {"type": "WikibaseItem", "relation": False},
        "make": "split-ended",
    },
]

# The block of one group of values.
BLOCK_RULES = [
    {"id": "media", "when": {"type": "CommonsMedia"}, "make": "media"},
    {"id": "coordinate", "when": {"type": "GlobeCoordinate"}, "make": "map"},
    {
        "id": "series",
        "when": {"type": "Quantity", "role": "series", "distinct_years": True},
        "make": "series",
    },
    {
        "id": "former",
        "when": {"type": "WikibaseItem", "former": True},
        "make": "former-list",
    },
    {
        "id": "few-pictures",
        "when": {"type": "WikibaseItem", "mostly_pictured": False},
        "make": "item-list",
    },
    {"id": "single", "when": {"type": "WikibaseItem", "few": True}, "make": "single"},
    {"id": "set", "when": {"type": "WikibaseItem"}, "make": "network"},
    {"id": "row", "when": {}, "make": "fact"},
]

# The order of the values of a set.
ORDER_RULES = [
    {"id": "billing", "when": {"role": "billing"}, "order": "references"},
    {"id": "pictured", "when": {}, "order": "pictured"},
]

# The first block of these primitives leads its spread.
LEAD = ("succession", "map")

# The slot of every other block.
PLACEMENT_RULES = [
    {"id": "single", "when": {"make": "single"}, "slot": "gallery"},
    {"id": "row", "when": {"primitive": "fact"}, "slot": "sheet"},
    {
        "id": "picture",
        "when": {"primitive": "media", "audio": False},
        "slot": "gallery",
    },
    {"id": "block", "when": {}, "slot": "stack"},
]

# The template, from the lead.
TEMPLATE_RULES = [
    {"id": "office", "when": {"lead": "succession"}, "template": "lead-beside"},
    {"id": "map", "when": {"lead": "map"}, "template": "hero-plus-stack"},
    {"id": "plain", "when": {}, "template": "columns"},
]


def match(rules, predicate):
    """The first rule whose conditions all hold. predicate(name) gives the
    value of a predicate; it is computed only when a rule asks for it."""
    for rule in rules:
        if all(holds(predicate(k), v) for k, v in rule["when"].items()):
            return rule
    return None


def holds(actual, wanted):
    if isinstance(actual, (set, frozenset)):
        return wanted in actual
    if isinstance(wanted, tuple):
        return actual in wanted
    return actual == wanted


class Composer:
    def __init__(self, content):
        self.c = content
        self.qid = content["qid"]
        self.sts = content["statements"]
        self.by_pid = {}
        for s in self.sts:
            self.by_pid.setdefault(s["pid"], []).append(s)
        self.claimed = set()
        # A value that is a class gets no picture: "lawyer", "species" and
        # "big city" have pictures of examples, not of the value.
        self.classes = set(content.get("value_classes", []))
        for s in self.sts:
            if s["type"] == "WikibaseItem" and entity_id(s["value"]) in self.classes:
                s["value_image"] = None
        peers = content.get("peers") or {}
        self.peer_count = len(peers.get("members", []))
        self.peer_freq = peers.get("frequency", {})
        self.layout_name = content.get("class_layout", "generic")
        self.layout = CONFIG["class_layouts"][self.layout_name]
        self.offices = set(ROLES["office"].values())

    # --------------------------------------------------------- statements

    def provenance(self, st):
        sets = self.c["references"].get(st["id"], [])
        if not sets:
            return "none"
        not_source = set(CONFIG["provenance"]["not_a_source"])
        return "sourced" if any(set(r) - not_source for r in sets) else "imported"

    def statement(self, st):
        """A statement as the composition shows it."""
        quals = [q for q in (qualifier_of(q) for q in st["qualifiers"]) if q]
        out = {
            "statement": st["id"],
            "pid": st["pid"],
            **value_of(st),
            "rank": st["rank"],
            "provenance": self.provenance(st),
        }
        if quals:
            out["qualifiers"] = quals
        if st["type"] == "WikibaseItem":
            out["image"] = (
                self.picture(
                    st["value_image"], logo=st.get("value_image_from") == "P154"
                )
                if st["value_image"]
                else None
            )
            if entity_id(st["value"]) in self.classes:
                out["class"] = True
        return out

    def take(self, sts):
        """Claim statements for one spread. A statement is shown once."""
        live = [s for s in sts if s["id"] not in self.claimed]
        self.claimed |= {s["id"] for s in live}
        return live

    def free(self, pid):
        return [s for s in self.by_pid.get(pid, []) if s["id"] not in self.claimed]

    def priority(self, pid, sts=()):
        p = 2 * self.peer_freq.get(pid, 0) / self.peer_count if self.peer_count else 0
        p += math.log2(1 + sum(1 for s in sts if self.provenance(s) == "sourced"))
        return round(p, 3)

    @staticmethod
    def ended(st):
        return any(q["pid"] == "P582" for q in st["qualifiers"])

    def current(self, sts):
        """Preferred rank first, then values without an end date, then the
        latest start."""
        return sorted(
            sts,
            key=lambda s: (
                s["rank"] != "preferred",
                self.ended(s),
                -(qual_year(s, "P580") or -(10**6)),
            ),
        )

    # -------------------------------------------------------------- media

    def picture(self, name, logo=False):
        info = self.c["commons"].get(name, {})
        low = name.lower()
        kind = (
            "audio"
            if low.endswith(AUDIO)
            else "video"
            if low.endswith(VIDEO)
            else "model3d"
            if low.endswith(".stl")
            else "drawing"
            if low.endswith(".svg")
            else "photo"
        )
        if logo and kind == "photo":
            kind = "drawing"  # a logo is shown whole, whatever its file format
        out = {"file": name, "kind": kind}
        if info.get("width") and info.get("height"):
            out["aspect"] = round(info["width"] / info["height"], 4)
            out["megapixels"] = round(info["width"] * info["height"] / 1e6, 2)
        for key in ("bytes", "url"):
            if info.get(key) and kind in ("video", "model3d", "audio"):
                out[key] = info[key]
        if info.get("assessments"):
            out["assessed"] = info["assessments"]
        out["credit"] = {
            "artist": info.get("artist", ""),
            "license": info.get("license", ""),
        }
        return out

    def media_block(self, st, role="run"):
        block = {
            "primitive": "media",
            "role": role,
            **self.statement(st),
            **self.picture(value_of(st)["file"]),
        }
        block.pop("image", None)
        return block

    def feature_score(self, st):
        pic = self.picture(value_of(st)["file"])
        if (
            pic["kind"] != "photo"
            or pic.get("megapixels", 0) < CONFIG["media"]["min_megapixels"]
        ):
            return None
        views = CONFIG["media"]["views"]
        assessed = sum(
            CONFIG["media"]["assessments"].get(a, 0) for a in pic.get("assessed", [])
        )
        if not assessed and st["pid"] not in views:
            return None
        pref = (
            (len(views) - views.index(st["pid"])) / len(views)
            if st["pid"] in views
            else 0
        )
        return 2 * assessed + 2 * pref + 0.5 * math.log2(pic["megapixels"])

    @staticmethod
    def visual(block):
        return bool(block) and (
            block.get("aspect")
            or block["primitive"] in ("map", "succession")
            or (block.get("image") or {}).get("aspect")
        )

    # ------------------------------------------------------------- blocks

    def fact(self, pid, sts, form=None):
        """A row of the fact sheet. Its form follows the shape of the data."""
        if form is None:
            form = "value"
            ends = {
                CONFIG["range_ends"].get(entity_id(q["value"]))
                for s in sts
                for q in s["qualifiers"]
            }
            if (
                sts[0]["type"] == "Quantity"
                and len(sts) == 2
                and {"min", "max"} <= ends
            ):
                form = "range"
            elif sts[0]["type"] == "Quantity" and len(sts) > 1:
                form = "variants"
            elif len(sts) > 6:
                form = "list"
        # A long list shows its first values (current first); the rest are
        # there, in a dialog.
        return {
            "primitive": "fact",
            "pid": pid,
            "form": form,
            "statements": [
                dict(self.statement(s), shown=i < TH["list_shown"])
                for i, s in enumerate(sts)
            ],
            "priority": self.priority(pid, sts),
        }

    def series(self, pid, sts):
        """Dated measurements of one property, when their years are distinct."""
        points = [(qual_year(s, "P585"), s) for s in sts]
        years = [y for y, _ in points if y]
        if len(sts) < 3 or len(years) != len(sts) or len(set(years)) != len(years):
            return None
        top = max(float(s["value"]) for s in sts)
        pts = sorted(points, key=lambda p: p[0])
        return {
            "primitive": "series",
            "pid": pid,
            "points": [
                {
                    "year": y,
                    "amount": float(s["value"]),
                    "fraction": round(float(s["value"]) / top, 4),
                    "statement": s["id"],
                    "provenance": self.provenance(s),
                }
                for y, s in pts
            ],
        }

    def tabular_series(self, pid, st):
        """A series from a Commons data file (P4179 tabular population): the
        year column and the first numeric column."""
        table = self.c.get("tabular", {}).get(st["value"])
        if not table:
            return None
        fields = table["fields"]
        yi = next(
            (i for i, f in enumerate(fields) if re.search(r"year|date|time", f, re.I)),
            None,
        )
        vi = next(
            (
                i
                for i, row in enumerate(zip(*table["rows"]))
                if i != yi
                and all(isinstance(v, (int, float)) for v in row if v is not None)
            ),
            None,
        )
        if yi is None or vi is None:
            return None
        pts = [
            (int(str(r[yi])[:4]), float(r[vi]))
            for r in table["rows"]
            if r[yi] is not None and r[vi] is not None
        ]
        if len(pts) < 3:
            return None
        pts.sort()
        top = max(v for _, v in pts)
        peak = max(pts, key=lambda p: p[1])
        return {
            "primitive": "series",
            "pid": pid,
            "statement": st["id"],
            "provenance": self.provenance(st),
            "source": {"file": table["title"], "license": table.get("license", "")},
            "points": [
                {
                    "year": y,
                    "amount": v,
                    "fraction": round(v / top, 4),
                    # With many points, only the ends and the peak carry numbers.
                    "labelled": len(pts) <= 12 or (y, v) in (pts[0], pts[-1], peak),
                }
                for y, v in pts
            ],
        }

    def map_block(self, st, spread_pids):
        block = {"primitive": "map", **self.statement(st)}
        area = self.c["quantities"].get(self.qid, {}).get("values", {}).get("P2046")
        if area:
            block["extent_m"] = round(math.sqrt(area["amount"]))
        markers = []
        for pid in ROLES["extreme_point"]:
            if pid in spread_pids:
                for s in self.take(self.free(pid)):
                    markers.append(
                        {
                            "pid": pid,
                            **value_of(s),
                            "statement": s["id"],
                            "provenance": self.provenance(s),
                        }
                    )
        if markers:
            block["markers"] = markers
        return block

    # -------------------------------------------------------------- rules

    def roles(self, pid):
        """The roles of a property (layouts.toml [roles])."""
        return {name for name, members in ROLES.items() if pid in members}

    def property_predicate(self, pid):
        values = {
            "office_title": lambda: pid in self.offices,
            "role": lambda: self.roles(pid),
        }
        return lambda name: values[name]()

    def group_predicate(self, pid, sts, former=False):
        values = {
            "type": lambda: sts[0]["type"],
            "role": lambda: self.roles(pid),
            "relation": lambda: "relation" in self.roles(pid),
            "former": lambda: former,
            "few": lambda: len(sts) <= 2,
            "mostly_pictured": lambda: (
                sum(1 for s in sts if s["value_image"]) >= TH["run_pictured"] * len(sts)
            ),
            "distinct_years": lambda: self.series(pid, sts) is not None,
        }
        return lambda name: values[name]()

    def blocks_for(self, pid, sts, spread_pids=()):
        """The blocks of the claimed statements of one property, by the rules,
        as (make, block) pairs."""
        groups = [(sts, False)]
        if sts[0]["type"] == "WikibaseItem":
            sts = self.current(sts)  # item values: current first
            groups = [(sts, False)]
            if match(SPLIT_RULES, self.group_predicate(pid, sts)):
                ended = [s for s in sts if self.ended(s)]
                groups = [(ended, True), ([s for s in sts if s not in ended], False)]
        out = []
        for group, former in groups:
            if group:
                rule = match(BLOCK_RULES, self.group_predicate(pid, group, former))
                out += [
                    (rule["make"], b)
                    for b in self.make(rule["make"], pid, group, spread_pids)
                ]
        return out

    def order_key(self, order):
        """The sort key of an order rule. Python sorts are stable, so equal
        values keep the order they had (current first, for items)."""
        if order == "references":
            return lambda s: (
                -len(self.c["references"].get(s["id"], [])),
                not s["value_image"],
                int(entity_id(s["value"])[1:]) if entity_id(s["value"]) else 10**12,
            )
        return lambda s: not s["value_image"]  # pictured values first

    def make(self, make, pid, sts, spread_pids):
        """The blocks that one rule makes of a group of values."""
        if make == "media":
            return [self.media_block(sts[0], "stack")]
        if make == "map":
            return [self.map_block(sts[0], spread_pids)]
        if make == "series":
            return [self.series(pid, sts)]
        if make == "former-list":
            return [{**self.fact(pid, sts, "list"), "former": True}]
        if make == "item-list":
            return [self.fact(pid, sts, "list" if len(sts) > 2 else None)]
        if make == "single":
            # Small values: each joins the shared gallery of its spread.
            p = self.priority(pid, sts)
            return [dict(self.statement(s), priority=p) for s in sts]
        if make == "network":
            order = match(ORDER_RULES, self.group_predicate(pid, sts))["order"]
            return [network.build(self, pid, sts, TH["set_max"], self.order_key(order))]
        return [self.fact(pid, sts)]

    def reverse_block(self, pids):
        """Works by the item, from statements on the works."""
        works = [r for r in self.c.get("reverse", []) if set(r["pids"]) & set(pids)]
        if not works:
            return None
        values = [
            {
                "item": r["item"],
                "via": r["pids"],
                "year": r["year"],
                "image": self.picture(r["image"]) if r["image"] else None,
            }
            for r in works
        ]
        # The same rule as for item sets: pictures only when most have one.
        pictured = sum(1 for v in values if v["image"])
        form = "run" if pictured >= TH["run_pictured"] * len(values) else "list"
        return {
            "primitive": "works",
            "pids": sorted({p for r in works for p in r["pids"]}),
            "form": form,
            "values": values,
            "shown_count": min(len(values), TH["list_shown"]),
            "priority": 1 + math.log2(1 + len(values)),
        }

    # ------------------------------------------------------------ spreads

    def spread(self, sp, slots, template, silent=None):
        out = {
            "id": sp["id"],
            "kind": sp["kind"],
            "concept": sp.get("concept"),
            "template": template,
            "slots": slots,
        }
        if sp.get("prose"):
            out["prose"] = {"concepts": sp["prose"]}
        for pid, sid in CONFIG["topics"].items():
            if sid == sp["id"]:
                for st in self.take(self.free(pid)):
                    out.setdefault("topics", []).append(self.statement(st))
        if silent:
            out["silent"] = silent
        return out

    def identity(self, sp):
        images = self.free("P18")
        hero = next(
            (s for s in images if s["rank"] == "preferred"),
            images[0] if images else None,
        )
        # The same order for every class: the title and description first,
        # then the banner, then the lead beside the picture.
        slots = {"head": {"primitive": "title", "item": self.qid}}
        banner = self.take(self.free("P948"))
        if banner:
            slots["band"] = self.media_block(banner[0], "banner")
        chain = []
        kinds = self.take(self.current(self.free("P31")))
        if kinds:
            chain.append(
                {
                    "pid": "P31",
                    "values": [
                        dict(self.statement(s), shown=i < TH["chain_max"])
                        for i, s in enumerate(kinds)
                    ],
                }
            )
        cont = self.c["container"]
        now = (
            [s for s in self.free(cont["pid"]) if entity_id(s["value"]) == cont["qid"]]
            if cont["pid"]
            else []
        )
        if now:
            chain.append(
                {
                    "pid": cont["pid"],
                    "values": [
                        self.statement(s) for s in self.take(self.current(now)[:1])
                    ],
                }
            )
        slots["lead"] = {
            "primitive": "prose",
            "source": "lead",
            "languages": sum(
                1
                for k in self.c["sitelinks"]
                if re.fullmatch(r"[a-z_-]+wiki", k)
                and k not in ("commonswiki", "specieswiki", "wikidatawiki", "metawiki")
            ),
        }
        if hero:
            slots["hero"] = self.media_block(self.take([hero])[0], "identity")
        slots["kind"] = {"primitive": "kind", "chain": chain}
        return self.spread(sp, slots, "identity")

    def facts(self, sp):
        """The properties a spread lists, each by the shape of its data and
        its role. The first picture-like block leads the spread."""
        pids = list(sp.get("pids", []))
        if sp.get("pictured"):
            # Also any item property whose values are mostly pictured.
            skip = (
                set(CONFIG["evidence"]["pids"])
                | set(ROLES["web"])
                | set(CONFIG["topics"])
                | {"P31"}
            )
            for pid, sts in self.by_pid.items():
                live = [s for s in sts if s["id"] not in self.claimed]
                if (
                    pid not in pids
                    and pid not in skip
                    and live
                    and live[0]["type"] == "WikibaseItem"
                    and sum(1 for s in live if s["value_image"])
                    >= TH["run_pictured"] * len(live)
                ):
                    pids.append(pid)
        made = []  # (make, block), in the order of the spread's properties
        for pid in pids:
            rule = match(PROPERTY_RULES, self.property_predicate(pid))
            if rule["make"] == "club":
                # All the crew properties of the spread: one block, by
                # person, where the first of them is.
                if not any(m == "credits" for m, _ in made):
                    crew = [p for p in pids if "crew" in self.roles(p)]
                    block = credits.build(self, crew, TH["set_max"])
                    if block:
                        made.append(("credits", block))
            elif rule["make"] == "succession":
                block = succession.build(
                    self, pid, ROLES["office"][pid], TH["succession_shown"]
                )
                if block:
                    made.append(("succession", block))
            elif rule["make"] == "values":
                sts = self.take(self.free(pid))
                if sts:
                    made += self.blocks_for(pid, sts, pids)
        # The age at death, from the two dates: an intrinsic fact.
        birth, death = self.by_pid.get("P569"), self.by_pid.get("P570")
        if birth and death:
            for _, r in made:
                if r.get("primitive") == "fact" and r["pid"] == "P570":
                    b, d = year_of(birth[0]["value"]), year_of(death[0]["value"])
                    bm, dm = birth[0]["value"][5:10], death[0]["value"][5:10]
                    r["age"] = {"pid": "P3629", "years": d - b - (1 if dm < bm else 0)}
        reverse = self.reverse_block(sp.get("reverse", []))
        if reverse:
            made.append(("works", reverse))
        if not made:
            return None
        slots, template = self.place(made)
        return self.spread(sp, slots, template)

    def place(self, made):
        """Slots and template of a facts spread, by the placement rules."""
        placed = {"stack": [], "gallery": [], "sheet": []}
        singles = []
        for make, block in made:
            facts = {
                "make": make,
                "primitive": block.get("primitive"),
                "audio": block.get("kind") == "audio",
            }
            slot = match(PLACEMENT_RULES, facts.get)["slot"]
            (singles if make == "single" else placed[slot]).append(block)
        slots = {}
        lead = next((b for b in placed["stack"] if b["primitive"] in LEAD), None)
        if lead:
            slots["hero"] = lead
        stack = [b for b in placed["stack"] if b is not lead]
        if stack:
            slots["stack"] = stack
        # Single pictures of different properties, of the item and of its
        # values: a slot of their own, before the sheet in reading order. A
        # template with named rows places it after the two columns.
        gallery = placed["gallery"]
        if singles:
            gallery.append(network.gallery(singles))
        if gallery:
            slots["gallery"] = gallery
        if placed["sheet"]:
            slots["sheet"] = placed["sheet"]
        lead_kind = lead["primitive"] if lead else None
        template = match(TEMPLATE_RULES, {"lead": lead_kind}.get)["template"]
        return slots, template

    def scale(self, sp):
        """The item against its container and its peers, and its series."""
        cont = self.c["container"]
        mode = CONFIG["containers"].get(cont["pid"] or "")
        qs = self.c["quantities"]
        mine = qs.get(self.qid, {}).get("values", {})
        theirs = qs.get(cont["qid"] or "", {}).get("values", {})
        pids = sp.get("pids", [])
        shared = [
            p
            for p in CONFIG["compare"]["pids"]
            if p in pids and p in mine and p in theirs and theirs[p]["amount"]
        ]
        slots, stack, silent = {}, [], []
        if mode and shared:
            measures = []
            for p in shared:
                st = self.current(self.by_pid[p])[0]
                measures.append(
                    {
                        "pid": p,
                        "ratio": round(mine[p]["amount"] / theirs[p]["amount"], 6),
                        "amount": mine[p]["amount"],
                        "whole": theirs[p]["amount"],
                        "unit": None if mine[p]["unit"] == "Q199" else mine[p]["unit"],
                        "statement": st["id"],
                        "provenance": self.provenance(st),
                    }
                )
            slots["hero"] = {
                "primitive": "scale",
                "mode": mode,
                "container": {"pid": cont["pid"], "item": cont["qid"]},
                "measures": measures,
            }
        else:
            silent.append({"primitive": "scale", "reason": "no_comparable_quantity"})
        stack += self.peers(pids)
        # The series: a Commons data file first, the dated statements else.
        tab = next(
            (s for p in ROLES["tabular"] if p in pids for s in self.free(p)), None
        )
        series = self.tabular_series(tab["pid"], tab) if tab else None
        rows = []
        if series:
            self.take([tab])
            # The file holds the same counts as the dated population
            # statements, from its own source. The current count stays as a
            # fact; the dated ones are shown by the file's series.
            counts = self.current(self.free("P1082"))
            if counts:
                rows.append(self.fact("P1082", counts[:1]))
                self.take(counts)
        for p in pids:
            if p in ROLES["series"] and self.free(p):
                block = self.series(p, self.free(p))
                if block:
                    self.take(self.free(p))
                    stack.append(block)
        if series:
            stack.insert(0, series)
        for p in pids:
            sts = self.take(self.free(p))
            if not sts:
                continue
            if sts[0]["type"] == "WikibaseItem":
                made = self.blocks_for(p, sts)
                stack += [
                    b for m, b in made if m != "single" and b["primitive"] != "fact"
                ]
                rows += [
                    b for m, b in made if m != "single" and b["primitive"] == "fact"
                ]
                # A small value is a row here: this spread has no gallery.
                rows += [
                    self.fact(p, [next(x for x in sts if x["id"] == b["statement"])])
                    for m, b in made
                    if m == "single"
                ]
            else:
                rows.append(self.fact(p, sts))
        if stack:
            slots["stack"] = stack
        if rows:
            slots["sheet"] = rows
        if not slots:
            return None
        return self.spread(sp, slots, "columns", silent)

    def peers(self, pids):
        peers = self.c.get("peers")
        if not peers:
            return []
        values = peers.get("quantities", {})
        own = self.c["quantities"].get(self.qid, {}).get("values", {})
        out = []
        for pid in CONFIG["compare"]["pids"]:
            members = [
                (q, r["values"][pid]["amount"], r)
                for q, r in values.items()
                if pid in r["values"]
            ]
            if pid not in own or len(members) < TH["peers"]:
                continue
            if self.qid not in {q for q, _, _ in members}:
                members.append((self.qid, own[pid]["amount"], {"image": None}))
            members.sort(key=lambda m: -m[1])
            top = members[0][1]
            rank = [q for q, _, _ in members].index(self.qid) + 1
            shown = {q for q, _, _ in members[:8]} | {self.qid}
            out.append(
                {
                    "primitive": "peers",
                    "pid": pid,
                    "class": peers["class"],
                    "container": peers.get("container"),
                    "rank": rank,
                    "of": len(members),
                    "members": [
                        {
                            "item": q,
                            "amount": v,
                            "fraction": round(v / top, 4),
                            "self": q == self.qid,
                            "shown": q in shown,
                        }
                        for q, v, r in members
                    ],
                }
            )
        return out[:2]

    def history(self, sp):
        """A time axis of history only: events of the item (by role) and the
        spans of relations that changed what it is (former countries, capital
        of). Dated measurements are series, never events."""
        tracks, years = {}, []
        for st in self.sts:
            pid = st["pid"]
            mark = None
            if pid in ROLES["event"]:
                if st["type"] == "Time" and year_of(st["value"]) is not None:
                    mark = {"year": year_of(st["value"])}
                elif qual_year(st, "P585") is not None:
                    mark = {"year": qual_year(st, "P585")}
                elif (
                    qual_year(st, "P580") is not None
                    or qual_year(st, "P582") is not None
                ):
                    mark = {
                        "start": qual_year(st, "P580"),
                        "end": qual_year(st, "P582"),
                    }
            elif pid in ROLES["history_span"] and (
                qual_year(st, "P580") is not None or qual_year(st, "P582") is not None
            ):
                mark = {"start": qual_year(st, "P580"), "end": qual_year(st, "P582")}
            if not mark:
                continue
            years += [y for y in mark.values() if y is not None]
            entry = {
                **mark,
                **{
                    k: v
                    for k, v in self.statement(st).items()
                    if k in ("statement", "item", "time", "precision", "provenance")
                },
            }
            tracks.setdefault(pid, []).append(entry)
        if len(set(years)) < TH["time_points"]:
            return (
                self.spread(
                    sp,
                    {},
                    "axis",
                    silent=[
                        {
                            "primitive": "time",
                            "reason": "below_threshold",
                            "points": len(set(years)),
                            "needs": TH["time_points"],
                        }
                    ],
                )
                if years
                or any(CONFIG["topics"].get(p) == sp["id"] for p in self.by_pid)
                else None
            )
        lo, hi = min(years), max(years)
        span = max(hi - lo, 1)
        for entries in tracks.values():
            for m in entries:
                for key in ("year", "start", "end"):
                    if m.get(key) is not None:
                        m[f"{key}_pos"] = round((m[key] - lo) / span, 4)
        # Events belong to this spread; a history span stays where its
        # relation is shown, and the axis repeats it on purpose.
        self.take([s for s in self.sts if s["pid"] in ROLES["event"]])
        self.take(
            [s for s in self.sts if s["pid"] in ROLES["history_span"] and self.ended(s)]
        )
        axis = {
            "primitive": "time",
            "from": lo,
            "to": hi,
            "tracks": [
                {
                    "pid": p,
                    "marks": sorted(
                        ms,
                        key=lambda m: m.get("year") or m.get("start") or m.get("end"),
                    ),
                }
                for p, ms in sorted(
                    tracks.items(),
                    key=lambda kv: min(
                        m.get("year") or m.get("start") or m.get("end") for m in kv[1]
                    ),
                )
            ],
        }
        return self.spread(sp, {"axis": axis}, "axis")

    def media(self, sp):
        sts = self.take(
            [
                s
                for s in self.sts
                if s["type"] == "CommonsMedia" and s["id"] not in self.claimed
            ]
        )
        if not sts:
            return None
        candidates = [
            (f, s)
            for f, s in ((self.feature_score(s), s) for s in sts)
            if f is not None
        ]
        slots = {}
        feature = max(candidates, key=lambda fs: fs[0])[1] if candidates else None
        if feature:
            slots["feature"] = self.media_block(feature, "feature")
        run = [
            self.media_block(s)
            for s in sts
            if s is not feature and self.picture(value_of(s)["file"])["kind"] != "audio"
        ]
        audio = [
            self.media_block(s, "audio")
            for s in sts
            if self.picture(value_of(s)["file"])["kind"] == "audio"
        ]
        order = {"photo": 0, "video": 1, "model3d": 2, "drawing": 3}
        run.sort(key=lambda b: (order.get(b["kind"], 4), -b.get("megapixels", 0)))
        if run:
            slots["run"] = run
        if audio:
            slots["audio"] = audio
        return self.spread(sp, slots, "feature-run")

    def evidence(self, sp):
        shown = [s for s in self.sts if s["type"] != "ExternalId"]
        counts = {"sourced": 0, "imported": 0, "none": 0}
        for s in shown:
            counts[self.provenance(s)] += 1
        web = [
            self.fact(p, self.take(self.free(p))) for p in ROLES["web"] if self.free(p)
        ]
        curation = [
            self.fact(p, self.take(self.free(p)))
            for p in CONFIG["evidence"]["pids"]
            if self.free(p)
        ]
        ids = self.take([s for s in self.sts if s["type"] == "ExternalId"])
        families = {}
        for site in self.c["sitelinks"]:
            m = re.match(
                r"^(.+?)(wiki|wikiquote|wikivoyage|wikinews|wikisource|wikibooks|wikiversity|wiktionary)$",
                site,
            )
            if m and site not in (
                "commonswiki",
                "specieswiki",
                "wikidatawiki",
                "metawiki",
                "mediawikiwiki",
            ):
                families.setdefault(
                    "wikipedia" if m.group(2) == "wiki" else m.group(2), []
                ).append(m.group(1))
        slots = {"summary": [{"primitive": "provenance", **counts}]}
        if web:
            slots["web"] = web
        slots["sheet"] = curation
        slots["projects"] = [
            {
                "primitive": "sitelinks",
                "families": {f: sorted(l) for f, l in families.items()},
            }
        ]
        slots["identifiers"] = [
            {
                "primitive": "identifiers",
                "values": [
                    {"pid": s["pid"], "string": s["value"], "statement": s["id"]}
                    for s in ids
                ],
            }
        ]
        peers = self.c.get("peers") or {}
        if self.peer_count >= TH["absence_min_peers"]:
            absent = [
                {
                    "primitive": "absent",
                    "pid": pid,
                    "peers_with_it": peers["frequency"][pid],
                    "peers": self.peer_count,
                    "class": peers["class"],
                    "datatype": datatype,
                }
                for pid, datatype in (peers.get("missing") or {}).items()
            ]
            if absent:
                slots["absent"] = absent
        return self.spread(sp, slots, "sheet")

    def more(self, sp):
        """Everything no spread listed: kept, demoted, collapsed."""
        free = {}
        for s in self.sts:
            if s["id"] not in self.claimed and s["type"] != "ExternalId":
                free.setdefault(s["pid"], []).append(s)
        rows = [self.fact(p, self.take(sts)) for p, sts in free.items()]
        rows.sort(key=lambda r: -r["priority"])
        return self.spread(sp, {"sheet": rows}, "collapsed") if rows else None

    # ------------------------------------------------------------- page

    def compose(self):
        builders = {
            "identity": self.identity,
            "facts": self.facts,
            "scale": self.scale,
            "history": self.history,
            "media": self.media,
            "more": self.more,
            "evidence": self.evidence,
        }
        spreads = self.layout["spreads"]
        # Evidence and "more" take what is left, so they are built last,
        # whatever their place on the page.
        built = {}
        for sp in sorted(
            spreads, key=lambda s: {"evidence": 1, "more": 2}.get(s["kind"], 0)
        ):
            built[sp["id"]] = builders[sp["kind"]](sp)
        out = [built[sp["id"]] for sp in spreads if built.get(sp["id"])]
        lost = [s["id"] for s in self.sts if s["id"] not in self.claimed]
        return {
            "schema": SCHEMA,
            "design": DESIGN,
            "qid": self.qid,
            "class_layout": self.layout_name,
            "spreads": out,
            "unshown": lost,
        }


def main(qid):
    content = json.loads(
        (HERE / "data" / "content" / f"{qid.lower()}.json").read_text()
    )
    composition = Composer(content).compose()
    out = HERE / "data" / "composition" / f"{qid.lower()}.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(composition, ensure_ascii=False, indent=1))
    print(
        f"{out.name}: {composition['class_layout']}, "
        f"{[s['id'] + ':' + s['template'] for s in composition['spreads']]}, "
        f"unshown {len(composition['unshown'])} ({out.stat().st_size:,} B)"
    )


if __name__ == "__main__":
    main(sys.argv[1])
