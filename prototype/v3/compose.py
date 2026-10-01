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
import urllib.parse
from pathlib import Path

HERE = Path(__file__).resolve().parent
CONFIG = tomllib.loads((HERE / "layouts.toml").read_text())
ROLES = CONFIG["roles"]
TH = CONFIG["thresholds"]
ENTITY = "http://www.wikidata.org/entity/"
SCHEMA = "composition/0.2"
DESIGN = "design/0.2"

AUDIO = (".ogg", ".oga", ".wav", ".mp3", ".flac", ".opus")
VIDEO = (".webm", ".ogv", ".mp4")


# ------------------------------------------------------------------ values


def entity_id(value):
    if (
        isinstance(value, str)
        and value.startswith(ENTITY)
        and re.fullmatch(r"[PQ]\d+", value[len(ENTITY) :])
    ):
        return value[len(ENTITY) :]
    return None


def year_of(iso):
    m = re.match(r"^([+-]?)0*(\d+)-", iso or "")
    if not m:
        return None
    return -int(m.group(2)) if m.group(1) == "-" else int(m.group(2))


def qual_year(st, pid):
    return next(
        (year_of(q["value"]) for q in st["qualifiers"] if q["pid"] == pid), None
    )


def value_of(st):
    """A statement value, as identifiers and raw values."""
    v, kind = st["value"], st["type"]
    if kind == "WikibaseItem":
        return {"item": entity_id(v)}
    if kind == "Quantity":
        return {
            "amount": float(v),
            "unit": None if st["unit"] in (None, "Q199") else st["unit"],
        }
    if kind == "Time":
        return {"time": v, "precision": st["precision"]}
    if kind == "Monolingualtext":
        return {"text": v, "lang": st["value_lang"]}
    if kind == "CommonsMedia":
        return {"file": urllib.parse.unquote(v.rsplit("/", 1)[-1])}
    if kind == "GlobeCoordinate":
        m = re.search(r"(?:entity/(Q\d+)> )?Point\(([-\d.]+) ([-\d.]+)\)", v)
        return {
            "lat": float(m.group(3)),
            "lon": float(m.group(2)),
            "globe": m.group(1) or "Q2",
        }
    if kind == "Url":
        return {"url": v}
    return {"string": v}


def qualifier_of(q):
    v = q["value"]
    if entity_id(v):
        return {"pid": q["pid"], "item": entity_id(v)}
    if re.match(r"^[+-]?\d{4,}-\d\d-\d\dT", v):
        return {"pid": q["pid"], "year": year_of(v)}
    if v.startswith("http://www.wikidata.org/.well-known/genid"):
        return None  # "some value": nothing to show
    if q.get("value_lang"):
        return {"pid": q["pid"], "text": v, "lang": q["value_lang"]}
    try:
        return {"pid": q["pid"], "amount": float(v)}
    except ValueError:
        return {"pid": q["pid"], "string": v}


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

    def succession(self, holder_pid):
        """An office and its holders: the office as the title, the holder now
        as the lead picture, the predecessors newest first, a few shown and
        the rest behind "+N"."""
        holders = self.current(self.free(holder_pid))
        if not holders:
            return None
        self.take(holders)
        office_pid = ROLES["office"][holder_pid]
        office = self.current(self.free(office_pid))
        office = self.take(office[:1])[0] if office else None
        now, before = holders[0], holders[1:]
        before.sort(
            key=lambda s: -(qual_year(s, "P580") or qual_year(s, "P582") or -(10**6))
        )
        return {
            "primitive": "succession",
            "pid": holder_pid,
            "office": self.statement(office) if office else None,
            "holder": self.statement(now),
            "before": [
                dict(self.statement(s), shown=i < TH["succession_shown"])
                for i, s in enumerate(before)
            ],
            "priority": self.priority(holder_pid, holders),
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

    def items_blocks(self, pid, sts):
        """Item values: history as a dated list, many values as a list, a
        pictured set, or small values for the shared gallery."""
        out, singles = [], []
        sts = self.current(sts)
        ended = [] if pid in ROLES["relation"] else [s for s in sts if self.ended(s)]
        if ended:
            block = self.fact(pid, ended, "list")
            block["former"] = True
            out.append(block)
            sts = [s for s in sts if s not in ended]
        if not sts:
            return out, singles
        pictured = sum(1 for s in sts if s["value_image"])
        if pictured < TH["run_pictured"] * len(sts):
            out.append(self.fact(pid, sts, "list" if len(sts) > 2 else None))
        elif len(sts) <= 2:
            singles += [
                dict(self.statement(s), priority=self.priority(pid, sts)) for s in sts
            ]
        else:
            # A large set shows its first pictures, pictured values first;
            # all the values are in "+N".
            sts = sorted(sts, key=lambda s: not s["value_image"])
            out.append(
                {
                    "primitive": "network",
                    "pid": pid,
                    "values": [
                        dict(self.statement(s), shown=i < TH["set_max"])
                        for i, s in enumerate(sts)
                    ],
                    "priority": self.priority(pid, sts),
                }
            )
        return out, singles

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
        blocks, singles, rows = [], [], []
        for pid in pids:
            if pid in self.offices or pid in ROLES["extreme_point"]:
                continue  # the office titles its holders; points go on the map
            if pid in ROLES["office"]:
                block = self.succession(pid)
                if block:
                    blocks.append(block)
                continue
            sts = self.take(self.free(pid))
            if not sts:
                continue
            kind = sts[0]["type"]
            if kind == "CommonsMedia":
                blocks.append(self.media_block(sts[0], "stack"))
            elif kind == "GlobeCoordinate":
                blocks.append(self.map_block(sts[0], pids))
            elif (
                kind == "Quantity" and pid in ROLES["series"] and self.series(pid, sts)
            ):
                blocks.append(self.series(pid, sts))
            elif kind == "WikibaseItem":
                b, s_ = self.items_blocks(pid, sts)
                blocks += [x for x in b if x["primitive"] != "fact"]
                rows += [x for x in b if x["primitive"] == "fact"]
                singles += s_
            else:
                rows.append(self.fact(pid, sts))
        # The age at death, from the two dates: an intrinsic fact.
        birth, death = self.by_pid.get("P569"), self.by_pid.get("P570")
        if birth and death:
            for r in rows:
                if r["pid"] == "P570":
                    b, d = year_of(birth[0]["value"]), year_of(death[0]["value"])
                    bm, dm = birth[0]["value"][5:10], death[0]["value"][5:10]
                    r["age"] = {"pid": "P3629", "years": d - b - (1 if dm < bm else 0)}
        reverse = self.reverse_block(sp.get("reverse", []))
        if reverse:
            blocks.append(reverse)
        gallery = None
        if singles:
            singles.sort(key=lambda v: (v.get("image") is None, -v["priority"]))
            gallery = {
                "primitive": "network",
                "role": "gallery",
                "values": singles,
                "priority": max(v["priority"] for v in singles),
            }
        if not blocks and not rows and not gallery:
            return None
        slots = {}
        lead = next(
            (b for b in blocks if b["primitive"] in ("succession", "map")), None
        )
        if lead:
            slots["hero"] = lead
            blocks = [b for b in blocks if b is not lead]
        if blocks:
            slots["stack"] = blocks
        if rows:
            slots["sheet"] = rows
        # Single pictures of different properties: a slot of their own, at
        # full width after the others, not at the end of the stack, where
        # they make the stack taller than the hero and the sheet.
        # A single picture of the item (a locator map, a seal) is one of them.
        pictures = [
            b
            for b in slots.get("stack", [])
            if b["primitive"] == "media" and b["kind"] != "audio"
        ]
        if pictures:
            slots["stack"] = [b for b in slots["stack"] if b not in pictures]
            if not slots["stack"]:
                del slots["stack"]
        if gallery or pictures:
            # Before the sheet in reading order: pictures first. A template
            # with named rows still places it after the two columns.
            sheet = slots.pop("sheet", None)
            slots["gallery"] = pictures + ([gallery] if gallery else [])
            if sheet:
                slots["sheet"] = sheet
        template = (
            "lead-beside"
            if lead and lead["primitive"] == "succession"
            else "hero-plus-stack"
            if lead
            else "columns"
        )
        return self.spread(sp, slots, template)

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
                b, s_ = self.items_blocks(p, sts)
                stack += [x for x in b if x["primitive"] != "fact"]
                rows += [x for x in b if x["primitive"] == "fact"]
                rows += [
                    self.fact(p, [next(x for x in sts if x["id"] == v["statement"])])
                    for v in s_
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
