use editor_crdt::Changeset;
use editor_macros::ffi;
use editor_model::{EditOp, PlainDoc};
use editor_resource::{ResourceRevision, ResourceSnapshot};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{EditorEvent, Message, SystemEvent};

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId {
    pub value: u64,
}

impl RequestId {
    pub(crate) const fn new(value: u64) -> Self {
        Self { value }
    }
}

#[ffi]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Revision {
    pub value: u64,
}

impl Revision {
    pub const INITIAL: Self = Self { value: 0 };

    pub const fn get(self) -> u64 {
        self.value
    }

    pub(crate) fn next(self) -> Self {
        Self {
            value: self.value.checked_add(1).expect("editor revision overflow"),
        }
    }
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandRejection {
    TargetNotFound,
    ParentNotFound,
    InvalidArgument,
    WrongNodeKind,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandOutcome {
    Applied,
    Rejected { reason: CommandRejection },
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestOutcome {
    pub request_id: RequestId,
    pub command_outcomes: Vec<CommandOutcome>,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TickResult {
    pub revision: Revision,
    pub events: Vec<EditorEvent>,
    pub request_outcomes: Vec<RequestOutcome>,
}

#[derive(Clone)]
pub struct ResourceUpdate {
    snapshot: Arc<ResourceSnapshot>,
    notices: Vec<SystemEvent>,
}

impl ResourceUpdate {
    pub fn new(snapshot: Arc<ResourceSnapshot>, notices: Vec<SystemEvent>) -> Self {
        Self { snapshot, notices }
    }

    pub fn snapshot(&self) -> &Arc<ResourceSnapshot> {
        &self.snapshot
    }

    pub fn revision(&self) -> ResourceRevision {
        self.snapshot.revision()
    }

    pub fn notices(&self) -> &[SystemEvent] {
        &self.notices
    }

    pub(crate) fn into_notices(self) -> Vec<SystemEvent> {
        self.notices
    }

    pub(crate) fn coalesce(&mut self, newer: Self) {
        self.snapshot = newer.snapshot;
        self.notices.extend(newer.notices);
    }
}

impl std::fmt::Debug for ResourceUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceUpdate")
            .field("revision", &self.revision())
            .field("notices", &self.notices)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ResourceUpdate {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.snapshot, &other.snapshot) && self.notices == other.notices
    }
}

// FIFO order comes only from an entry's position in `Editor::queue`.
// `RequestId` exists solely to address a request outcome and the `tickThrough`
// boundary. Non-request entries need no ID because no caller awaits their
// individual outcome; they are still ordered and applied like every other entry.
pub(crate) enum QueueEntry {
    Request {
        id: RequestId,
        messages: Vec<Message>,
    },
    Resource(ResourceUpdate),
    Remote(Changeset<EditOp>),
    SetDocument(PlainDoc),
    InsertTemplate(PlainDoc),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{
        ImageNodeAttr, NodeAttr, PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode,
    };
    use editor_resource::{
        RawTextReplacementRule, Resource, ResourceSource, ThemeVariant,
        prepare_text_replacement_rules,
    };

    use crate::{CommandOutcome, Editor, EditorError, InsertionOp, Message, NodeOp, Revision};

    fn editor_with_empty_paragraph() -> Editor {
        let (state, ..) = state! {
            doc { root { p: paragraph { text("") } } }
            selection: (p, 0)
        };
        Editor::new_test(state)
    }

    fn insert(text: &str) -> Message {
        Message::Insertion {
            op: InsertionOp::Text { text: text.into() },
        }
    }

    fn unexpected_failure() -> Message {
        Message::Node {
            op: NodeOp::SetAttr {
                id: Dot::ROOT,
                attr: NodeAttr::Image {
                    attr: ImageNodeAttr::Proportion(50),
                },
            },
        }
    }

    fn replacement_update(source: &mut ResourceSource, substitute: &str) -> super::ResourceUpdate {
        let prepared = prepare_text_replacement_rules(vec![RawTextReplacementRule {
            id: "abc".into(),
            match_pattern: "abc".into(),
            substitute: substitute.into(),
            regex: false,
        }]);
        let snapshot = source
            .set_text_replacement_rules(prepared)
            .expect("replacement rules change");
        super::ResourceUpdate::new(snapshot, Vec::new())
    }

    fn theme_update(source: &mut ResourceSource) -> super::ResourceUpdate {
        let snapshot = source
            .set_theme_variant(ThemeVariant::DarkBlack)
            .expect("theme changes");
        super::ResourceUpdate::new(snapshot, vec![crate::SystemEvent::ThemeVariantChanged])
    }

    fn text(editor: &Editor) -> String {
        editor
            .state()
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .inline_text()
    }

    fn invalid_plain_doc() -> PlainDoc {
        PlainDoc {
            root: PlainNodeEntry {
                node: PlainNode::Paragraph(PlainParagraphNode {}),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: Vec::new(),
            },
        }
    }

    #[test]
    fn tick_through_drains_exact_request_prefix() {
        let mut editor = editor_with_empty_paragraph();
        let a = editor.enqueue_request(vec![insert("a")]).unwrap();
        let b = editor
            .enqueue_request(vec![insert("b"), insert("B")])
            .unwrap();
        let c = editor.enqueue_request(vec![insert("c")]).unwrap();

        let through_b = editor.tick_through(b).unwrap();

        assert_eq!(through_b.revision.get(), 1);
        assert_eq!(
            through_b
                .request_outcomes
                .iter()
                .map(|outcome| outcome.request_id)
                .collect::<Vec<_>>(),
            vec![a, b]
        );
        assert_eq!(through_b.request_outcomes[1].command_outcomes.len(), 2);
        assert_eq!(text(&editor), "abB");

        let later = editor.tick().unwrap().unwrap();
        assert_eq!(later.revision.get(), 2);
        assert_eq!(later.request_outcomes[0].request_id, c);
        assert_eq!(text(&editor), "abBc");
    }

    #[test]
    fn tick_through_rejects_an_absent_request_without_mutating_the_editor() {
        let mut editor = editor_with_empty_paragraph();
        let processed = editor.enqueue_request(vec![insert("a")]).unwrap();
        editor.tick_through(processed).unwrap();
        let queued = editor.enqueue_request(vec![insert("b")]).unwrap();

        assert!(matches!(
            editor.tick_through(processed),
            Err(EditorError::RequestNotQueued { request_id }) if request_id == processed
        ));
        assert_eq!(editor.revision(), Revision { value: 1 });
        assert_eq!(editor.queued_entry_count_for_test(), 1);
        assert_eq!(text(&editor), "a");

        editor.tick_through(queued).unwrap();
        assert_eq!(text(&editor), "ab");
    }

    #[test]
    fn tick_through_includes_intermediate_resource_update_and_leaves_later_work() {
        let mut source = ResourceSource::new_test();
        let resource = Arc::new(Mutex::new(Resource::from_snapshot(source.snapshot())));
        let (state, ..) = state! {
            doc { root { p: paragraph { text("") } } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        editor.enqueue_request(vec![insert("abc")]).unwrap();
        editor.receive_resource_update(replacement_update(&mut source, "X"));
        let b = editor.enqueue_request(vec![insert("abc")]).unwrap();
        editor.enqueue_request(vec![insert("abc")]).unwrap();
        editor.receive_resource_update(replacement_update(&mut source, "Y"));

        editor.tick_through(b).unwrap();

        assert_eq!(text(&editor), "abcX");
        assert_eq!(editor.resource_apply_count_for_test(), 1);
        assert_eq!(editor.queued_entry_count_for_test(), 2);

        editor.tick().unwrap().unwrap();
        assert_eq!(text(&editor), "abcXX");
        assert_eq!(editor.resource_apply_count_for_test(), 2);
        editor.apply(insert("abc"));
        assert_eq!(text(&editor), "abcXXY");
    }

    #[test]
    fn tick_revision_increments_once_for_all_requests_in_the_prefix() {
        let mut editor = editor_with_empty_paragraph();
        let a = editor.enqueue_request(vec![insert("a")]).unwrap();
        let b = editor.enqueue_request(vec![insert("b")]).unwrap();

        let first = editor.tick().unwrap().unwrap();

        assert_eq!(first.revision.get(), 1);
        assert_eq!(
            first
                .request_outcomes
                .iter()
                .map(|outcome| outcome.request_id)
                .collect::<Vec<_>>(),
            vec![a, b]
        );
        assert!(editor.tick().unwrap().is_none());

        let c = editor.enqueue_request(vec![insert("c")]).unwrap();
        let second = editor.tick().unwrap().unwrap();
        assert_eq!(second.revision.get(), 2);
        assert_eq!(second.request_outcomes[0].request_id, c);
    }

    #[test]
    fn remote_item_preserves_request_order_without_creating_an_outcome() {
        let mut editor = editor_with_empty_paragraph();
        let duplicate_remote = editor
            .state()
            .graph()
            .changesets_as_vec()
            .into_iter()
            .next()
            .unwrap();
        let a = editor.enqueue_request(vec![insert("a")]).unwrap();
        editor.receive_remote_changeset(duplicate_remote);
        let b = editor.enqueue_request(vec![insert("b")]).unwrap();

        let result = editor.tick().unwrap().unwrap();

        assert_eq!(
            result
                .request_outcomes
                .iter()
                .map(|outcome| outcome.request_id)
                .collect::<Vec<_>>(),
            vec![a, b]
        );
        assert_eq!(text(&editor), "ab");
    }

    #[test]
    fn duplicate_remote_changeset_is_a_complete_no_op() {
        let mut editor = editor_with_empty_paragraph();
        let duplicate_remote = editor
            .state()
            .graph()
            .changesets_as_vec()
            .into_iter()
            .next()
            .unwrap();

        editor.receive_remote_changeset(duplicate_remote);

        assert!(editor.tick().unwrap().is_none());
        assert_eq!(editor.revision(), Revision::INITIAL);
    }

    #[test]
    fn empty_request_is_rejected_without_queuing_work() {
        let mut editor = editor_with_empty_paragraph();

        assert!(matches!(
            editor.enqueue_request(Vec::new()),
            Err(EditorError::EmptyRequest)
        ));
        assert_eq!(editor.queued_entry_count_for_test(), 0);
        assert!(editor.tick().unwrap().is_none());
        assert_eq!(editor.revision(), Revision::INITIAL);
    }

    #[test]
    fn non_root_set_document_is_rejected_before_admission() {
        let mut editor = editor_with_empty_paragraph();
        let before = editor.state().to_plain();

        assert!(matches!(
            editor.set_doc(invalid_plain_doc()),
            Err(EditorError::General { .. })
        ));
        assert_eq!(editor.state().to_plain(), before);
        assert_eq!(editor.queued_entry_count_for_test(), 0);
        assert!(editor.tick().unwrap().is_none());
        assert_eq!(editor.revision(), Revision::INITIAL);
    }

    #[test]
    fn non_root_template_is_rejected_before_admission() {
        let mut editor = editor_with_empty_paragraph();
        let before = editor.state().to_plain();

        assert!(matches!(
            editor.insert_template_fragment(invalid_plain_doc()),
            Err(EditorError::General { .. })
        ));
        assert_eq!(editor.state().to_plain(), before);
        assert_eq!(editor.queued_entry_count_for_test(), 0);
        assert!(editor.tick().unwrap().is_none());
        assert_eq!(editor.revision(), Revision::INITIAL);
    }

    #[test]
    fn set_document_emits_render_invalidated() {
        let mut editor = editor_with_empty_paragraph();
        let (replacement, ..) = state! {
            doc { root { p: paragraph { text("replacement") } } }
            selection: (p, 0)
        };
        editor.set_doc(replacement.to_plain()).unwrap();

        let result = editor.tick().unwrap().unwrap();

        assert!(
            result
                .events
                .iter()
                .any(|event| matches!(event, crate::EditorEvent::RenderInvalidated))
        );
    }

    #[test]
    fn set_document_and_template_apply_in_fifo_order() {
        let mut editor = editor_with_empty_paragraph();
        let (replacement, ..) = state! {
            doc { root { p: paragraph { text("replacement") } } }
            selection: (p, 0)
        };
        let (template, ..) = state! {
            doc { root { p: paragraph { text("template") } } }
            selection: (p, 0)
        };
        editor.set_doc(replacement.to_plain()).unwrap();
        editor
            .insert_template_fragment(template.to_plain())
            .unwrap();

        let result = editor.tick().unwrap().unwrap();

        assert_eq!(result.revision, Revision { value: 1 });
        assert_eq!(text(&editor), "template");
        assert_eq!(editor.queued_entry_count_for_test(), 0);
    }

    #[test]
    fn ordered_resource_update_applies_between_requests() {
        let mut source = ResourceSource::new_test();
        let resource = Arc::new(Mutex::new(Resource::from_snapshot(source.snapshot())));
        let (state, ..) = state! {
            doc { root { p: paragraph { text("") } } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        editor.enqueue_request(vec![insert("abc")]).unwrap();
        editor.receive_resource_update(replacement_update(&mut source, "X"));
        editor.enqueue_request(vec![insert("abc")]).unwrap();

        editor.tick().unwrap().unwrap();

        assert_eq!(text(&editor), "abcX");
    }

    #[test]
    fn coalesced_resource_update_refreshes_local_resource_once() {
        let mut source = ResourceSource::new_test();
        let resource = Arc::new(Mutex::new(Resource::from_snapshot(source.snapshot())));
        let (state, ..) = state! {
            doc { root { p: paragraph { text("") } } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        editor.receive_resource_update(replacement_update(&mut source, "X"));
        editor.receive_resource_update(replacement_update(&mut source, "Y"));

        assert_eq!(editor.queued_entry_count_for_test(), 1);
        editor.tick().unwrap().unwrap();
        assert_eq!(editor.resource_apply_count_for_test(), 1);
        editor.apply(insert("abc"));
        assert_eq!(text(&editor), "Y");
    }

    #[test]
    fn expected_command_rejection_does_not_fail_later_commands() {
        let mut editor = editor_with_empty_paragraph();
        let request = editor
            .enqueue_request(vec![
                Message::Node {
                    op: NodeOp::Delete {
                        id: Dot::new(99, 99),
                    },
                },
                insert("ok"),
            ])
            .unwrap();

        let result = editor.tick().unwrap().unwrap();

        assert_eq!(result.request_outcomes[0].request_id, request);
        assert!(matches!(
            result.request_outcomes[0].command_outcomes[0],
            CommandOutcome::Rejected {
                reason: super::CommandRejection::TargetNotFound
            }
        ));
        assert_eq!(
            result.request_outcomes[0].command_outcomes[1],
            CommandOutcome::Applied
        );
        assert_eq!(text(&editor), "ok");
    }

    #[test]
    fn unexpected_tick_failure_returns_no_result_or_revision() {
        let mut editor = editor_with_empty_paragraph();
        editor
            .enqueue_request(vec![insert("unversioned"), unexpected_failure()])
            .unwrap();

        assert!(editor.tick().is_err());
        assert_eq!(editor.revision(), Revision::INITIAL);
        assert_eq!(text(&editor), "unversioned");
    }

    #[test]
    fn tick_through_excludes_a_later_unexpected_failure() {
        let mut editor = editor_with_empty_paragraph();
        let b = editor.enqueue_request(vec![insert("b")]).unwrap();
        editor.enqueue_request(vec![unexpected_failure()]).unwrap();

        assert!(editor.tick_through(b).is_ok());
        assert_eq!(text(&editor), "b");
        assert!(editor.tick().is_err());
    }

    #[test]
    fn tick_through_leaves_later_typed_entries_queued() {
        let mut source = ResourceSource::new_test();
        let resource = Arc::new(Mutex::new(Resource::from_snapshot(source.snapshot())));
        let (state, ..) = state! {
            doc { root { p: paragraph { text("") } } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);
        let request = editor.enqueue_request(vec![insert("abc")]).unwrap();
        editor.receive_resource_update(replacement_update(&mut source, "X"));
        let reset = editor.state().to_plain();
        editor.set_doc(reset).unwrap();

        editor.tick_through(request).unwrap();

        assert_eq!(text(&editor), "abc");
        assert_eq!(editor.resource_apply_count_for_test(), 0);
        assert_eq!(editor.queued_entry_count_for_test(), 2);

        editor.tick().unwrap().unwrap();
        assert_eq!(editor.resource_apply_count_for_test(), 1);
        assert_eq!(text(&editor), "");
    }

    #[test]
    fn tick_through_fails_when_an_unexpected_failure_precedes_the_target() {
        let mut editor = editor_with_empty_paragraph();
        editor.enqueue_request(vec![unexpected_failure()]).unwrap();
        let b = editor.enqueue_request(vec![insert("b")]).unwrap();

        assert!(editor.tick_through(b).is_err());
        assert_eq!(editor.revision(), Revision::INITIAL);
    }

    #[test]
    fn set_document_prevents_resource_coalescing_across_it() {
        let mut source = ResourceSource::new_test();
        let resource = Arc::new(Mutex::new(Resource::from_snapshot(source.snapshot())));
        let (state, ..) = state! {
            doc { root { p: paragraph { text("") } } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        editor.receive_resource_update(replacement_update(&mut source, "X"));
        let reset = editor.state().to_plain();
        editor.set_doc(reset).unwrap();
        editor.receive_resource_update(replacement_update(&mut source, "Y"));

        assert_eq!(editor.queued_entry_count_for_test(), 3);
    }

    #[test]
    fn text_rule_resource_update_does_not_invalidate_render() {
        let mut source = ResourceSource::new_test();
        let resource = Arc::new(Mutex::new(Resource::from_snapshot(source.snapshot())));
        let (state, ..) = state! {
            doc { root { p: paragraph { text("") } } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);
        let signature = editor.page_render_signature(0);
        editor.receive_resource_update(replacement_update(&mut source, "X"));

        let result = editor.tick().unwrap().unwrap();

        assert!(
            !result
                .events
                .iter()
                .any(|event| matches!(event, crate::EditorEvent::RenderInvalidated))
        );
        assert_eq!(
            editor.page_render_signature(0),
            signature,
            "text replacement rules do not change rendered pixels"
        );
    }

    #[test]
    fn theme_resource_update_emits_render_invalidated() {
        let mut source = ResourceSource::new_test();
        let resource = Arc::new(Mutex::new(Resource::from_snapshot(source.snapshot())));
        let (state, ..) = state! {
            doc { root { p: paragraph { text("") } } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);
        editor.receive_resource_update(theme_update(&mut source));

        let result = editor.tick().unwrap().unwrap();

        assert!(
            result
                .events
                .iter()
                .any(|event| matches!(event, crate::EditorEvent::RenderInvalidated))
        );
    }

    #[test]
    fn duplicate_resource_snapshot_does_not_replay_notices() {
        let mut source = ResourceSource::new_test();
        let resource = Arc::new(Mutex::new(Resource::from_snapshot(source.snapshot())));
        let (state, ..) = state! {
            doc { root { p: paragraph { text("") } } }
            selection: (p, 0)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);
        let update = theme_update(&mut source);

        editor.receive_resource_update(update.clone());
        editor.tick().unwrap().unwrap();
        let revision = editor.revision();

        editor.receive_resource_update(update);
        let duplicate = editor.tick().unwrap();

        assert_eq!(editor.resource_apply_count_for_test(), 1);
        assert!(duplicate.is_none());
        assert_eq!(editor.revision(), revision);
    }
}
