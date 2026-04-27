use editor_model::DocumentAttrs;
use editor_state::State;

use crate::{Step, StepError, StepOutput};

pub(crate) fn apply(state: &State, new_attrs: &DocumentAttrs) -> Result<StepOutput, StepError> {
    let mut new_state = state.clone();
    new_state.doc = new_state.doc.with_attrs(new_attrs.clone());

    Ok(StepOutput {
        state: new_state,
        validations: vec![],
    })
}

pub(crate) fn inverse(old_attrs: DocumentAttrs, new_attrs: DocumentAttrs) -> Step {
    Step::SetDocumentAttrs {
        old: new_attrs,
        new: old_attrs,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::*;

    use crate::*;

    #[test]
    fn set_document_attrs_apply() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let new_attrs = DocumentAttrs {
            layout_mode: LayoutMode::Paginated {
                page_width: 595.0,
                page_height: 842.0,
                page_margin_top: 72.0,
                page_margin_bottom: 72.0,
                page_margin_left: 72.0,
                page_margin_right: 72.0,
            },
        };

        let step = Step::SetDocumentAttrs {
            old: DocumentAttrs::default(),
            new: new_attrs.clone(),
        };

        let output = step.apply(&state).unwrap();

        assert_eq!(output.state.doc.attrs().layout_mode, new_attrs.layout_mode);
    }

    #[test]
    fn set_document_attrs_inverse_roundtrip() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let new_attrs = DocumentAttrs {
            layout_mode: LayoutMode::Continuous { max_width: 800.0 },
        };

        let step = Step::SetDocumentAttrs {
            old: DocumentAttrs::default(),
            new: new_attrs,
        };

        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(
            state3.doc.attrs().layout_mode,
            state.doc.attrs().layout_mode
        );
    }
}
