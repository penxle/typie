use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub fn handle_input(&mut self, text: &str) -> Vec<Effect> {
        let mut effects = Vec::new();

        if self.state.preedit.is_some() {
            effects.extend(self.transact(|tr| tr.complete_preedit()));
        }

        if let Some(surround_effects) = self.try_auto_surround(text) {
            effects.extend(surround_effects);
            return effects;
        }

        effects.extend(self.transact(|tr| {
            tr.delete_selection()?;
            tr.normalize()?;
            tr.insert_text(text)?;
            tr.try_text_replacement(text.len())?;
            Ok(true)
        }));

        effects
    }

    pub fn handle_replace_backward(&mut self, length: usize, text: &str) -> Vec<Effect> {
        let pending_styles = self.state.pending_styles.clone();

        let mut effects = self.transact(|tr| {
            for _ in 0..length {
                tr.delete_text_backward()?;
            }
            Ok(true)
        });

        if self.state.pending_styles != pending_styles {
            self.state.pending_styles = pending_styles;
            effects.push(Effect::PendingStylesChanged);
        }

        effects.extend(self.transact(|tr| {
            tr.insert_text(text)?;
            tr.try_text_replacement(text.len())?;
            Ok(true)
        }));
        effects
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{ItalicStyle, Style};
    use crate::runtime::Message;
    use crate::types::Affinity;

    #[test]
    fn replace_backward_preserves_pending_style_across_mixed_deleted_styles() {
        let mut p = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "a" }
                }
            }
            selection { (p, 1) }
        };

        runtime.update(Message::ToggleStyle {
            style: Style::Italic(ItalicStyle {}),
        });
        runtime.update(Message::Input {
            text: "b".to_string(),
        });
        runtime.update(Message::ReplaceBackward {
            length: 2,
            text: "x".to_string(),
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [italic()]) { "x" }
                }
            }
            selection { (p, 1, Affinity::Upstream) }
        };
        assert_state_eq!(runtime.state(), expected);

        assert!(
            runtime
                .state()
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::Italic(_))),
            "pending style should preserve italic after replacement, got: {:?}",
            runtime.state().pending_styles
        );
    }
}
