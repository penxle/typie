use std::ops::Range;

use super::run::ProseRun;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProseText {
    text: String,
    plain_len: usize,
    pub(super) runs: Vec<ProseRun>,
}

#[derive(Debug, Clone, Copy)]
enum Bias {
    Left,
    Right,
}

impl ProseText {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn to_flat_range(&self, range: Range<usize>) -> Option<Range<usize>> {
        if range.start > range.end {
            return None;
        }
        if range.end > self.plain_len {
            return None;
        }
        if self.runs.is_empty() {
            return None;
        }

        if range.start == range.end {
            let p = self.locate(range.start, Bias::Right)?;
            return Some(p..p);
        }

        let start = self.locate(range.start, Bias::Right)?;
        let end = self.locate(range.end, Bias::Left)?;
        Some(start..end)
    }

    fn locate(&self, plain_pos: usize, bias: Bias) -> Option<usize> {
        let idx = self
            .runs
            .partition_point(|r| r.plain_range.end <= plain_pos);

        if idx == self.runs.len() {
            let last = self.runs.last()?;
            if plain_pos == last.plain_range.end {
                return Some(last.flat_start + last.plain_range.len());
            }
            return None;
        }

        let run = &self.runs[idx];

        if plain_pos == run.plain_range.start && idx > 0 {
            let prev = &self.runs[idx - 1];
            return Some(match bias {
                Bias::Left => prev.flat_start + prev.plain_range.len(),
                Bias::Right => run.flat_start,
            });
        }

        Some(run.flat_start + (plain_pos - run.plain_range.start))
    }

    pub(super) fn from_parts(text: String, runs: Vec<ProseRun>, plain_len: usize) -> Self {
        debug_assert_eq!(plain_len, text.chars().count());
        Self {
            text,
            plain_len,
            runs,
        }
    }

    #[cfg(test)]
    pub(crate) fn check_invariants(&self, doc: &editor_model::Doc) -> Result<(), String> {
        use crate::DocFlatExt;
        use crate::ResolvedPosition;
        use crate::ResolvedPositionFlatExt;

        let actual_len = self.text.chars().count();
        if actual_len != self.plain_len {
            return Err(format!(
                "plain_len cache desync: cached {} vs actual {}",
                self.plain_len, actual_len
            ));
        }
        let total = self.plain_len;
        let flat_size = doc.flat_size();

        if self.runs.is_empty() {
            if total != 0 {
                return Err(format!("text len {total} but runs is empty"));
            }
            return Ok(());
        }

        for r in &self.runs {
            if r.plain_range.start >= r.plain_range.end {
                return Err(format!("zero or inverted plain run: {:?}", r.plain_range));
            }
        }
        for w in self.runs.windows(2) {
            if w[0].plain_range.end != w[1].plain_range.start {
                return Err(format!(
                    "gap or overlap: {:?} -> {:?}",
                    w[0].plain_range, w[1].plain_range
                ));
            }
        }

        if self.runs.first().unwrap().plain_range.start != 0 {
            return Err("first run does not start at 0".into());
        }
        if self.runs.last().unwrap().plain_range.end != total {
            return Err(format!(
                "last run ends at {}, expected {total}",
                self.runs.last().unwrap().plain_range.end
            ));
        }

        for r in &self.runs {
            let end_flat = r.flat_start + r.plain_range.len();
            if end_flat > flat_size {
                return Err(format!(
                    "run flat_start {} + len {} > flat_size {}",
                    r.flat_start,
                    r.plain_range.len(),
                    flat_size
                ));
            }
            if ResolvedPosition::from_flat(doc, r.flat_start).is_none() {
                return Err(format!("from_flat({}) returned None", r.flat_start));
            }
            if ResolvedPosition::from_flat(doc, end_flat).is_none() {
                return Err(format!("from_flat({end_flat}) returned None (run end)"));
            }
        }

        if total > 0 {
            let r = self
                .to_flat_range(0..total)
                .ok_or_else(|| "to_flat_range(0..total) returned None".to_string())?;
            if ResolvedPosition::from_flat(doc, r.start).is_none() {
                return Err(format!(
                    "full-range start flat {} not from_flat-able",
                    r.start
                ));
            }
            if ResolvedPosition::from_flat(doc, r.end).is_none() {
                return Err(format!("full-range end flat {} not from_flat-able", r.end));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::DocProseExt;
    use editor_macros::doc;

    #[test]
    fn to_flat_range_within_single_run() {
        let (doc, ..) = doc! { root { paragraph { text("hello world") } } };
        let prose = doc.prose();
        assert_eq!(prose.to_flat_range(1..5), Some(2..6));
    }

    #[test]
    fn to_flat_range_full_text() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        let prose = doc.prose();
        let len = prose.text().chars().count();
        assert_eq!(prose.to_flat_range(0..len), Some(1..3));
    }

    #[test]
    fn to_flat_range_out_of_bounds_returns_none() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        let prose = doc.prose();
        assert_eq!(prose.to_flat_range(0..3), None);
        assert_eq!(prose.to_flat_range(5..5), None);
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn to_flat_range_inverted_returns_none() {
        let (doc, ..) = doc! { root { paragraph { text("hi") } } };
        let prose = doc.prose();
        assert_eq!(prose.to_flat_range(2..1), None);
    }

    #[test]
    fn to_flat_range_on_empty_doc_returns_none() {
        let (doc, ..) = doc! { root {} };
        let prose = doc.prose();
        assert_eq!(prose.to_flat_range(0..0), None);
    }

    #[test]
    fn to_flat_range_empty_range_returns_zero_width() {
        let (doc, ..) = doc! { root { paragraph { text("hello") } } };
        let prose = doc.prose();
        assert_eq!(prose.to_flat_range(3..3), Some(4..4));
    }

    #[test]
    fn to_flat_range_empty_range_at_flat_gap_is_not_inverted() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                horizontal_rule {}
                paragraph { text("b") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\nb");
        let r = prose.to_flat_range(2..2).expect("empty range must map");
        assert!(r.start <= r.end, "got inverted range {:?}", r);
        assert_eq!(r.start, r.end);
        assert_eq!(r, 4..4);
    }

    #[test]
    fn to_flat_range_across_text_node_split() {
        let (doc, ..) = doc! { root { paragraph { text("hel") text("lo") } } };
        let prose = doc.prose();
        assert_eq!(prose.to_flat_range(1..4), Some(2..5));
    }

    #[test]
    fn to_flat_range_across_simple_block_boundary() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                paragraph { text("b") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\nb");
        assert_eq!(prose.to_flat_range(0..4), Some(1..5));
        assert_eq!(prose.to_flat_range(1..3), Some(2..4));
        assert_eq!(prose.to_flat_range(0..2), Some(1..3));
        assert_eq!(prose.to_flat_range(2..4), Some(3..5));
    }

    #[test]
    fn to_flat_range_across_block_boundary_with_atom_gap() {
        let (doc, ..) = doc! {
            root {
                paragraph { text("a") }
                horizontal_rule {}
                paragraph { text("b") }
            }
        };
        let prose = doc.prose();
        assert_eq!(prose.text(), "a\n\nb");
        assert_eq!(prose.to_flat_range(0..4), Some(1..6));
        assert_eq!(prose.to_flat_range(1..3), Some(2..5));
        assert_eq!(prose.to_flat_range(1..2), Some(2..3));
        assert_eq!(prose.to_flat_range(2..3), Some(4..5));
        assert_eq!(prose.to_flat_range(0..2), Some(1..3));
        let r = prose.to_flat_range(2..2).expect("must map");
        assert!(r.start <= r.end);
        assert_eq!(r, 4..4);
    }

    #[test]
    fn invariants_hold_for_fixtures() {
        macro_rules! check {
            ($name:expr, $doc:expr) => {{
                let (doc, ..) = $doc;
                let prose = doc.prose();
                prose
                    .check_invariants(&doc)
                    .unwrap_or_else(|e| panic!("invariant failed for {}: {e}", $name));
            }};
        }

        check!("empty doc", doc! { root {} });
        check!(
            "single paragraph",
            doc! { root { paragraph { text("hi") } } }
        );
        check!(
            "two paragraphs",
            doc! { root { paragraph { text("a") } paragraph { text("b") } } }
        );
        check!(
            "hard break inside",
            doc! { root { paragraph { text("a") hard_break {} text("b") } } }
        );
        check!(
            "blockquote",
            doc! { root { blockquote { paragraph { text("x") } } paragraph { text("y") } } }
        );
        check!(
            "atom between",
            doc! { root { paragraph { text("a") } horizontal_rule {} paragraph { text("b") } } }
        );
        check!(
            "empty middle",
            doc! { root { paragraph { text("a") } paragraph {} paragraph { text("b") } } }
        );
        check!(
            "empty text node",
            doc! { root { paragraph { text("") } paragraph { text("a") } } }
        );
        check!("multibyte", doc! { root { paragraph { text("한글ñ") } } });
        check!(
            "nested list",
            doc! { root { bullet_list { list_item { paragraph { text("a") } bullet_list { list_item { paragraph { text("b") } } } } } } }
        );
        check!(
            "hard break only",
            doc! { root { paragraph { hard_break {} } paragraph { text("a") } } }
        );
        check!(
            "boundary then hard break",
            doc! { root { paragraph { text("a") } paragraph { hard_break {} text("b") } } }
        );
        check!(
            "all empty paragraphs",
            doc! { root { paragraph {} paragraph {} } }
        );
    }
}
