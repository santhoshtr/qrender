//! Layer 2: the composer. It reads the content of one item and the
//! configuration, never a label, so its output is the same in every
//! language.
//!
//! The `Composer` holds the bookkeeping (each statement is shown once) and
//! the derived facts: pure functions of the content that the rules and the
//! primitives use.

pub mod blocks;
pub mod facts;
pub mod picture;
pub mod place;
pub mod spreads;

use std::collections::HashSet;

use content::{Cited, Content, Pid, Qid, QualifierValue, Rank, Statement, StatementId, Value};
use indexmap::IndexMap;

use crate::config::{ClassLayout, Config, SpreadSpec};
use crate::schema::{
    Block, COMPOSITION_SCHEMA, Composition, DESIGN, Dropped, Field, ProseConcepts, Provenance,
    QualifierShown, Silence, Spread, StatementRef, ValueShown, Wanted, round_to,
};
use crate::vocab::{Role, Slot, SpreadKind, Template};
use crate::wikidata::{END_TIME, POINT_IN_TIME, START_TIME};

/// "No year": sorts before every real year.
const NO_YEAR: i64 = -1_000_000;
/// The pages of Commons, by name.
const COMMONS: &str = "https://commons.wikimedia.org/wiki/";

pub struct Composer<'a> {
    pub content: &'a Content,
    pub config: &'a Config,
    pub layout_name: &'a str,
    pub layout: &'a ClassLayout,
    /// The statements of each property, in content order.
    by_pid: IndexMap<Pid, Vec<&'a Statement>>,
    claimed: HashSet<&'a StatementId>,
    /// Value items that are classes: never pictured.
    classes: HashSet<Qid>,
    /// Properties that name the office of a holder property.
    offices: HashSet<Pid>,
    peer_count: usize,
    /// The texts shown with an item value: item statement -> the text
    /// statements whose words name its item (motto -> motto text).
    texts: IndexMap<&'a StatementId, Vec<&'a Statement>>,
}

impl<'a> Composer<'a> {
    pub fn new(content: &'a Content, config: &'a Config) -> Self {
        let mut by_pid: IndexMap<Pid, Vec<&Statement>> = IndexMap::new();
        for statement in &content.statements {
            by_pid.entry(statement.pid).or_default().push(statement);
        }
        let layouts = &config.layouts.class_layouts;
        let (layout_name, layout) = layouts
            .get_key_value(content.class_layout.as_str())
            .or_else(|| layouts.get_key_value("generic"))
            .expect("validated at load: a generic layout exists");
        Self {
            content,
            config,
            layout_name,
            layout,
            by_pid,
            claimed: HashSet::new(),
            classes: content.value_classes.iter().copied().collect(),
            offices: config.layouts.roles.office.values().copied().collect(),
            peer_count: content.peers.as_ref().map_or(0, |p| p.members.len()),
            texts: IndexMap::new(),
        }
    }

    // ---------------------------------------------------------- bookkeeping

    /// Claim statements for one spread. A statement is shown once, so the
    /// ones claimed before are left out.
    pub fn take(
        &mut self,
        statements: impl IntoIterator<Item = &'a Statement>,
    ) -> Vec<&'a Statement> {
        statements
            .into_iter()
            .filter(|s| self.claimed.insert(&s.id))
            .collect()
    }

    /// The statements of a property that no spread has claimed.
    pub fn free(&self, pid: Pid) -> Vec<&'a Statement> {
        self.by_pid
            .get(&pid)
            .into_iter()
            .flatten()
            .copied()
            .filter(|s| !self.claimed.contains(&s.id))
            .collect()
    }

    pub fn is_claimed(&self, statement: &Statement) -> bool {
        self.claimed.contains(&statement.id)
    }

    /// The statements that no spread has claimed, in content order.
    pub fn unclaimed(&self) -> Vec<&'a Statement> {
        self.content
            .statements
            .iter()
            .filter(|s| !self.claimed.contains(&s.id))
            .collect()
    }

    pub fn statements_of(&self, pid: Pid) -> &[&'a Statement] {
        self.by_pid.get(&pid).map_or(&[], Vec::as_slice)
    }

    /// Every property of the item, with its statements, in content order.
    pub fn properties(&self) -> impl Iterator<Item = (Pid, &[&'a Statement])> {
        self.by_pid.iter().map(|(p, s)| (*p, s.as_slice()))
    }

    // ---------------------------------------------------------------- page

    /// The composition of the item. Every spread of the class layout, in
    /// its order; "evidence" and "more" are built last, because they take
    /// what the other spreads left.
    pub fn compose(mut self) -> Composition {
        let layout = self.layout;
        let set_aside = self.set_aside();
        self.join_texts();
        let mut order: Vec<&SpreadSpec> = layout.spreads.iter().collect();
        order.sort_by_key(|s| match s.kind {
            SpreadKind::Evidence => 1,
            SpreadKind::More => 2,
            _ => 0,
        });
        let mut built: IndexMap<&str, Spread> = IndexMap::new();
        let mut dropped: IndexMap<&str, Vec<Silence>> = IndexMap::new();
        for spec in order {
            match spreads::build(&mut self, spec) {
                Ok(spread) => {
                    built.insert(&spec.id, spread);
                }
                Err(silent) => {
                    dropped.insert(&spec.id, silent);
                }
            }
        }
        let mut composition = Composition {
            schema: COMPOSITION_SCHEMA,
            design: DESIGN,
            qid: self.content.qid,
            class_layout: self.layout_name.to_string(),
            spreads: layout
                .spreads
                .iter()
                .filter_map(|s| built.shift_remove(s.id.as_str()))
                .map(|mut spread| {
                    spread.slots = std::mem::take(&mut spread.slots)
                        .into_iter()
                        .map(|(slot, blocks)| {
                            let blocks = crate::primitives::sounds::group(
                                blocks,
                                &self.content.language_codes,
                            );
                            (slot, crate::primitives::pictures::group(slot, blocks))
                        })
                        .collect();
                    spread
                })
                .collect(),
            dropped: layout
                .spreads
                .iter()
                .filter_map(|s| {
                    let silent = dropped.shift_remove(s.id.as_str())?;
                    Some(Dropped {
                        id: s.id.clone(),
                        silent,
                        wanted: self.wanted(s),
                    })
                })
                .collect(),
            set_aside,
            citations: IndexMap::new(),
            unshown: self.unclaimed().iter().map(|s| s.id.clone()).collect(),
        };
        composition.citations = cited(&composition, &self.content.citations);
        composition
    }

    /// Claims the values of the best-rank properties that are not of the
    /// best rank: the normal values of a property that has a preferred one.
    fn set_aside(&mut self) -> Vec<StatementRef> {
        let mut out = Vec::new();
        for pid in self.config.layouts.roles.list(Role::BestRank) {
            let statements = self.statements_of(*pid);
            if statements.iter().any(|s| s.rank == Rank::Preferred) {
                let normal: Vec<_> = statements
                    .iter()
                    .copied()
                    .filter(|s| s.rank != Rank::Preferred)
                    .collect();
                for s in self.take(normal) {
                    out.push(self.statement(s));
                }
            }
        }
        out
    }

    /// Claims each text that names an item value of its paired property
    /// (motto text, motto), and joins it to the first statement with that
    /// item: the value shows the words.
    fn join_texts(&mut self) {
        for (item_pid, text_pid) in &self.config.layouts.roles.text {
            let items = self.free(*item_pid);
            for text in self.free(*text_pid) {
                let Some(q) = self.content.text_items.get(&text.id) else {
                    continue;
                };
                if let Some(item) = items.iter().find(|s| s.value.item() == Some(*q)) {
                    self.take([text]);
                    self.texts.entry(&item.id).or_default().push(text);
                }
            }
        }
    }

    /// A spread with its slots. The "... of topic" statements of this
    /// spread are claimed here, as its topics.
    pub fn spread(
        &mut self,
        spec: &SpreadSpec,
        slots: IndexMap<Slot, Vec<Block>>,
        template: Template,
        silent: Vec<Silence>,
    ) -> Spread {
        let mut topics = Vec::new();
        for (pid, spread) in &self.config.layouts.topics {
            if *spread == spec.id {
                let statements = self.free(*pid);
                for s in self.take(statements) {
                    // A topic is an article: shown with its description
                    // and its picture, also when it is a class ("history
                    // of Germany"): the picture is not of an example.
                    let mut topic = self.statement(s);
                    topic.described = topic.value.item().is_some();
                    topic.class = false;
                    topic.image = s
                        .value_image
                        .as_ref()
                        .map(|i| self.picture(&i.file, i.logo));
                    topics.push(topic);
                }
            }
        }
        Spread {
            id: spec.id.clone(),
            kind: spec.kind,
            dimension: spec.dimension,
            concept: spec.concept,
            template,
            slots,
            prose: (!spec.prose.is_empty()).then(|| ProseConcepts {
                concepts: spec.prose.clone(),
            }),
            topics,
            silent,
            wanted: self.wanted(spec),
        }
    }

    /// The first wanted property of a spread that the item has no
    /// statement for, and that the page has a field for.
    fn wanted(&self, spec: &SpreadSpec) -> Option<Wanted> {
        spec.wanted.iter().find_map(|pid| {
            if !self.statements_of(*pid).is_empty() {
                return None;
            }
            let field = Field::of(self.content.wanted.get(pid)?)?;
            Some(Wanted { pid: *pid, field })
        })
    }

    // -------------------------------------------------------- derived facts

    /// The statement has ended (it has an end time qualifier).
    pub fn ended(statement: &Statement) -> bool {
        statement.qualifiers.iter().any(|q| q.pid == *END_TIME)
    }

    /// The statements in "current first" order: preferred rank first, then
    /// the ones that have not ended, then the latest start. The sort is
    /// stable, so equal statements keep their content order.
    pub fn current(&self, mut statements: Vec<&'a Statement>) -> Vec<&'a Statement> {
        statements.sort_by_key(|s| {
            (
                s.rank != Rank::Preferred,
                Self::ended(s),
                -qualifier_year(s, *START_TIME).unwrap_or(NO_YEAR),
            )
        });
        statements
    }

    /// Sourced when a reference has a property that is not in the
    /// `not_a_source` list; imported when every reference only points back
    /// into a Wikimedia project; none without references.
    pub fn provenance(&self, statement: &Statement) -> Provenance {
        let Some(sets) = self.content.references.get(&statement.id) else {
            return Provenance::None;
        };
        if sets.is_empty() {
            return Provenance::None;
        }
        let not_a_source = &self.config.layouts.provenance.not_a_source;
        if sets
            .iter()
            .any(|set| set.iter().any(|p| !not_a_source.contains(p)))
        {
            Provenance::Sourced
        } else {
            Provenance::Imported
        }
    }

    /// The number of references of a statement to a real source (not only
    /// "imported from Wikimedia project").
    pub fn sources(&self, statement: &Statement) -> usize {
        let not_a_source = &self.config.layouts.provenance.not_a_source;
        self.content
            .references
            .get(&statement.id)
            .map_or(0, |sets| {
                sets.iter()
                    .filter(|set| set.iter().any(|p| !not_a_source.contains(p)))
                    .count()
            })
    }

    /// The hashes of the references of a statement that cite a source (in
    /// the citations of the content).
    pub fn citations(&self, statement: &Statement) -> Vec<String> {
        self.content
            .statement_citations
            .get(&statement.id)
            .cloned()
            .unwrap_or_default()
    }

    /// How much a property matters: how many peers have it, and how many of
    /// its statements are sourced.
    pub fn priority(&self, pid: Pid, statements: &[&Statement]) -> f64 {
        let peers = match &self.content.peers {
            Some(peers) if self.peer_count > 0 => {
                2.0 * f64::from(peers.frequency.get(&pid).copied().unwrap_or(0))
                    / self.peer_count as f64
            }
            _ => 0.0,
        };
        let sourced = statements
            .iter()
            .filter(|s| self.provenance(s) == Provenance::Sourced)
            .count();
        round_to(peers + (1.0 + sourced as f64).log2(), 3)
    }

    pub fn has_role(&self, pid: Pid, role: Role) -> bool {
        self.config.roles(pid).has(role)
    }

    /// The property names the office of a holder property (P1313).
    pub fn is_office_title(&self, pid: Pid) -> bool {
        self.offices.contains(&pid)
    }

    pub fn is_class(&self, item: Qid) -> bool {
        self.classes.contains(&item)
    }

    /// The value is a class whose picture would show an example only. Not
    /// for a property whose classes are shown by their symbol (the symbol
    /// role: a religion).
    pub fn is_class_value(&self, pid: Pid, item: Qid) -> bool {
        self.is_class(item) && !self.has_role(pid, Role::Symbol)
    }

    /// The value of a statement as the composition shows it. The name of a
    /// Commons page is a link to it.
    fn value(&self, statement: &Statement) -> ValueShown {
        let page = |namespace: &str, name: &str| ValueShown::Link {
            text: name.to_string(),
            href: format!("{COMMONS}{namespace}{}", name.replace(' ', "_")),
        };
        match &statement.value {
            Value::String { string } if self.has_role(statement.pid, Role::CommonsCategory) => {
                page("Category:", string)
            }
            Value::String { string } if self.has_role(statement.pid, Role::CommonsGallery) => {
                page("", string)
            }
            value => ValueShown::from(value),
        }
    }

    /// A statement as the composition shows it.
    /// An item value with joined texts has their words and their
    /// qualifiers, the best provenance of them all, and the citations of
    /// them all.
    pub fn statement(&self, statement: &Statement) -> StatementRef {
        let item = statement.value.item();
        let class = item.is_some_and(|q| self.is_class_value(statement.pid, q));
        let texts = self.texts.get(&statement.id).map_or(&[][..], Vec::as_slice);
        let words = texts.iter().filter_map(|t| match &t.value {
            Value::Monolingualtext { text, lang } => QualifierShown::new(
                t.pid,
                &QualifierValue::Text {
                    text: text.clone(),
                    lang: lang.clone(),
                },
            ),
            _ => None,
        });
        let others = std::iter::once(statement)
            .chain(texts.iter().copied())
            .flat_map(|s| &s.qualifiers)
            .filter_map(|q| QualifierShown::new(q.pid, &q.value));
        let mut qualifiers: Vec<QualifierShown> = Vec::new();
        for q in words.chain(others) {
            if !qualifiers.contains(&q) {
                qualifiers.push(q);
            }
        }
        let mut citations: Vec<String> = Vec::new();
        for hash in std::iter::once(statement)
            .chain(texts.iter().copied())
            .flat_map(|s| self.citations(s))
        {
            if !citations.contains(&hash) {
                citations.push(hash);
            }
        }
        StatementRef {
            statement: statement.id.clone(),
            pid: statement.pid,
            value: self.value(statement),
            rank: statement.rank,
            provenance: std::iter::once(statement)
                .chain(texts.iter().copied())
                .map(|s| self.provenance(s))
                .min()
                .expect("one statement at least"),
            citations,
            qualifiers,
            texts: texts.iter().map(|t| t.id.clone()).collect(),
            image: self.picture_of(statement),
            class,
            described: item.is_some() && !class && self.has_role(statement.pid, Role::Described),
            shown: None,
        }
    }
}

/// The citations that the blocks of a composition cite: the hashes in
/// their `citations` lists. (The identifiers block cites none.)
fn cited(composition: &Composition, all: &IndexMap<String, Cited>) -> IndexMap<String, Cited> {
    fn walk<'v>(value: &'v serde_json::Value, out: &mut HashSet<&'v str>) {
        match value {
            serde_json::Value::Object(o) => {
                for (key, v) in o {
                    match (key.as_str(), v) {
                        ("citations", serde_json::Value::Array(hashes)) => {
                            out.extend(hashes.iter().filter_map(|h| h.as_str()));
                        }
                        _ => walk(v, out),
                    }
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let json = serde_json::to_value(composition).expect("a composition serializes");
    let mut hashes = HashSet::new();
    walk(&json, &mut hashes);
    all.iter()
        .filter(|(hash, _)| hashes.contains(hash.as_str()))
        .map(|(hash, c)| (hash.clone(), c.clone()))
        .collect()
}

/// The year of a measurement: its point in time; else its start time (a
/// value that holds from then: a minimum wage); else its end time.
pub fn measurement_year(statement: &Statement) -> Option<i64> {
    qualifier_year(statement, *POINT_IN_TIME)
        .or_else(|| qualifier_year(statement, *START_TIME))
        .or_else(|| qualifier_year(statement, *END_TIME))
}

/// The year of the first qualifier of a property, when it is a date.
pub fn qualifier_year(statement: &Statement, qualifier: Pid) -> Option<i64> {
    statement
        .qualifiers
        .iter()
        .find(|q| q.pid == qualifier)
        .and_then(|q| q.value.year())
}

#[cfg(test)]
pub(crate) fn pid(id: &str) -> Pid {
    id.parse().expect("a property id in the code")
}

#[cfg(test)]
pub(crate) mod test_content {
    //! Small hand-made items for the unit tests.

    use content::Content;
    use serde_json::{Value, json};

    /// An item from statements in the content JSON form. Each statement
    /// needs only `pid` and `value`; the id is made from its position.
    pub fn content(statements: Value, extra: Value) -> Content {
        let statements: Vec<Value> = statements
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let mut s = s.clone();
                let o = s.as_object_mut().unwrap();
                o.entry("id")
                    .or_insert_with(|| json!(format!("Q1-0000-{i:04}")));
                o.entry("rank").or_insert_with(|| json!("normal"));
                s
            })
            .collect();
        let mut c = json!({
            "schema": "content/0.1",
            "qid": "Q1",
            "statements": statements,
            "references": {},
            "container": {"pid": null, "qid": null},
            "quantities": {},
            "peers": null,
            "commons": {},
            "value_classes": [],
            "sitelinks": {},
            "class_layout": "generic",
            "reverse": [],
            "tabular": {},
        });
        for (k, v) in extra.as_object().unwrap() {
            c[k] = v.clone();
        }
        serde_json::from_value(c).unwrap()
    }

    pub fn item(pid: &str, qid: &str) -> Value {
        json!({"pid": pid, "value": {"type": "WikibaseItem", "item": qid}})
    }
}

#[cfg(test)]
mod tests {
    use super::test_content::{content, item};
    use super::*;
    use crate::schema::MediaKind;
    use serde_json::json;

    fn config() -> &'static Config {
        Config::embedded()
    }

    #[test]
    fn take_claims_once() {
        let c = content(
            json!([item("P31", "Q5"), item("P31", "Q6"), item("P17", "Q7")]),
            json!({}),
        );
        let mut k = Composer::new(&c, config());
        let p31 = pid("P31");
        let first = k.free(p31);
        assert_eq!(first.len(), 2);
        assert_eq!(k.take(first[..1].to_vec()).len(), 1);
        assert_eq!(k.free(p31).len(), 1);
        // Claimed before: not claimed again.
        assert_eq!(k.take(first).len(), 1);
        assert!(k.free(p31).is_empty());
        assert_eq!(k.unclaimed().len(), 1);
    }

    #[test]
    fn current_first() {
        let c = content(
            json!([
                {"pid": "P17", "value": {"type": "WikibaseItem", "item": "Q10"},
                 "qualifiers": [{"pid": "P580", "value": {"type": "time", "time": "1900-01-01T00:00:00Z"}},
                                {"pid": "P582", "value": {"type": "time", "time": "1950-01-01T00:00:00Z"}}]},
                {"pid": "P17", "value": {"type": "WikibaseItem", "item": "Q11"},
                 "qualifiers": [{"pid": "P580", "value": {"type": "time", "time": "1950-01-01T00:00:00Z"}}]},
                {"pid": "P17", "value": {"type": "WikibaseItem", "item": "Q12"},
                 "qualifiers": [{"pid": "P580", "value": {"type": "time", "time": "1990-01-01T00:00:00Z"}}]},
                {"pid": "P17", "value": {"type": "WikibaseItem", "item": "Q13"}, "rank": "preferred"},
            ]),
            json!({}),
        );
        let k = Composer::new(&c, config());
        let order: Vec<_> = k
            .current(k.free(pid("P17")))
            .iter()
            .map(|s| s.value.item().unwrap().to_string())
            .collect();
        // Preferred, then not ended with the latest start, then ended.
        assert_eq!(order, ["Q13", "Q12", "Q11", "Q10"]);
        assert!(Composer::ended(&c.statements[0]));
        assert!(!Composer::ended(&c.statements[1]));
    }

    #[test]
    fn provenance_by_reference_properties() {
        let c = content(
            json!([
                item("P31", "Q5"),
                item("P31", "Q6"),
                item("P31", "Q7"),
                item("P31", "Q8")
            ]),
            json!({"references": {
                "Q1-0000-0000": [["P248", "P813"]],
                "Q1-0000-0001": [["P143"], ["P143", "P813"]],
                "Q1-0000-0002": [],
                // One real source among Wikipedia imports: sourced.
                "Q1-0000-0003": [["P143"], ["P854", "P813"]],
            }}),
        );
        let k = Composer::new(&c, config());
        let p: Vec<_> = c.statements.iter().map(|s| k.provenance(s)).collect();
        assert_eq!(
            p,
            [
                Provenance::Sourced,
                Provenance::Imported,
                Provenance::None,
                Provenance::Sourced
            ]
        );
    }

    #[test]
    fn priority_from_peers_and_sources() {
        let c = content(
            json!([item("P31", "Q5"), item("P31", "Q6")]),
            json!({
                "references": {"Q1-0000-0000": [["P248"]], "Q1-0000-0001": [["P248"]]},
                "peers": {"class": "Q515", "container_pid": null, "container": null,
                          "members": ["Q2", "Q3", "Q4", "Q5"], "frequency": {"P31": 3},
                          "quantities": {}},
            }),
        );
        let k = Composer::new(&c, config());
        let sts: Vec<_> = c.statements.iter().collect();
        // 2 * 3/4 + log2(1 + 2)
        assert_eq!(k.priority(pid("P31"), &sts), round_to(1.5 + 3f64.log2(), 3));
        assert_eq!(k.priority(pid("P17"), &[]), 0.0);
    }

    #[test]
    fn statement_shape() {
        let c = content(
            json!([
                {"pid": "P26", "value": {"type": "WikibaseItem", "item": "Q20"},
                 "value_image": {"file": "Portrait.jpg"},
                 "qualifiers": [{"pid": "P580", "value": {"type": "time", "time": "1958-06-14T00:00:00Z"}},
                                {"pid": "P1534", "value": {"type": "somevalue"}}]},
                {"pid": "P106", "value": {"type": "WikibaseItem", "item": "Q40348"},
                 "value_image": {"file": "Lawyer.jpg"}},
                {"pid": "P144", "value": {"type": "WikibaseItem", "item": "Q30"}},
            ]),
            json!({"value_classes": ["Q40348"],
                   "commons": {"Portrait.jpg": {"width": 300, "height": 400, "bytes": 1,
                                                "url": "u", "artist": "A", "license": "CC0"}}}),
        );
        let k = Composer::new(&c, config());
        let spouse = serde_json::to_value(k.statement(&c.statements[0])).unwrap();
        assert_eq!(
            spouse,
            json!({
                "statement": "Q1-0000-0000", "pid": "P26", "item": "Q20", "rank": "normal",
                "provenance": "none",
                "qualifiers": [{"pid": "P580", "year": 1958}],
                "image": {"file": "Portrait.jpg", "kind": "photo", "aspect": 0.75,
                          "megapixels": 0.12, "credit": {"artist": "A", "license": "CC0"}},
            })
        );
        // A class value: no picture, marked as a class.
        let lawyer = k.statement(&c.statements[1]);
        assert!(lawyer.class && lawyer.image.is_none());
        // A work (role "described") carries its description.
        assert!(k.statement(&c.statements[2]).described);
    }

    #[test]
    fn a_motto_and_its_text_are_one_value() {
        let c = content(
            json!([
                {"pid": "P1546", "value": {"type": "WikibaseItem", "item": "Q191587"},
                 "qualifiers": [{"pid": "P5102", "value": {"type": "entity", "entity": "Q712144"}}]},
                {"pid": "P1451", "value": {"type": "Monolingualtext", "text": "E Pluribus Unum", "lang": "la"},
                 "qualifiers": [{"pid": "P5102", "value": {"type": "entity", "entity": "Q712144"}},
                                {"pid": "P2441", "value": {"type": "text", "text": "Out of many, one", "lang": "en"}}]},
                {"pid": "P1451", "value": {"type": "Monolingualtext", "text": "All within your reach", "lang": "en"}},
            ]),
            json!({"text_items": {"Q1-0000-0001": "Q191587"},
                   "references": {"Q1-0000-0001": [["P143"]]}}),
        );
        let mut k = Composer::new(&c, config());
        k.join_texts();
        // The matched text is claimed; the other text stays.
        assert_eq!(k.free(pid("P1451")).len(), 1);
        let motto = serde_json::to_value(k.statement(&c.statements[0])).unwrap();
        assert_eq!(
            motto,
            json!({
                "statement": "Q1-0000-0000", "pid": "P1546", "item": "Q191587", "rank": "normal",
                // The best provenance of the two.
                "provenance": "imported",
                // The words first; a qualifier of both statements once.
                "qualifiers": [{"pid": "P1451", "text": "E Pluribus Unum", "lang": "la"},
                               {"pid": "P5102", "item": "Q712144"},
                               {"pid": "P2441", "text": "Out of many, one", "lang": "en"}],
                "texts": ["Q1-0000-0001"],
            })
        );
    }

    #[test]
    fn a_spread_asks_for_its_first_missing_wanted_property() {
        let wanted = |datatypes: serde_json::Value| {
            let c = content(
                json!([item("P31", "Q5"), item("P21", "Q6581097")]),
                json!({"class_layout": "person", "wanted": datatypes}),
            );
            let composition = Composer::new(&c, config()).compose();
            let life = composition.spreads.iter().find(|s| s.id == "life").unwrap();
            life.wanted.as_ref().map(|w| (w.pid.to_string(), w.field))
        };
        // The item has P21 (sex or gender): P569 (date of birth) is next.
        assert_eq!(
            wanted(json!({"P569": "time", "P19": "wikibase-item"})),
            Some(("P569".to_string(), Field::Date))
        );
        // A datatype with no field is not asked for: the next one is.
        assert_eq!(
            wanted(json!({"P569": "globe-coordinate", "P19": "wikibase-item"})),
            Some(("P19".to_string(), Field::Item))
        );
        assert_eq!(wanted(json!({})), None);
    }

    #[test]
    fn the_head_picture_is_the_image_else_the_logo() {
        let file =
            |pid: &str, f: &str| json!({"pid": pid, "value": {"type": "CommonsMedia", "file": f}});
        let hero = |statements| {
            let c = content(statements, json!({}));
            let composition = Composer::new(&c, config()).compose();
            let Some(Block::Media(m)) = composition.spreads[0].slots[&Slot::Hero].first() else {
                panic!("a head picture");
            };
            (m.statement.pid, m.picture.kind)
        };
        let (logo, kind) = hero(json!([file("P154", "Logo.png")]));
        assert_eq!(logo, pid("P154"));
        assert_eq!(kind, MediaKind::Drawing, "a logo is shown whole");
        let (image, _) = hero(json!([file("P154", "Logo.png"), file("P18", "View.jpg")]));
        assert_eq!(image, pid("P18"));
    }
}
