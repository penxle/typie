use editor_model::{Modifier, ModifierType};

pub fn is_inheritable(ty: ModifierType) -> bool {
    match ty {
        ModifierType::Bold
        | ModifierType::Italic
        | ModifierType::Underline
        | ModifierType::Strikethrough
        | ModifierType::FontFamily
        | ModifierType::FontSize
        | ModifierType::FontWeight
        | ModifierType::TextColor
        | ModifierType::BackgroundColor
        | ModifierType::LetterSpacing
        | ModifierType::Link => true,
        ModifierType::Alignment
        | ModifierType::LineHeight
        | ModifierType::BlockGap
        | ModifierType::ParagraphIndent
        | ModifierType::Ruby => false,
    }
}

pub fn is_block_level(ty: ModifierType) -> bool {
    matches!(
        ty,
        ModifierType::Alignment
            | ModifierType::LineHeight
            | ModifierType::BlockGap
            | ModifierType::ParagraphIndent
    )
}

pub fn merge_with_inheritance(
    parent: &[Modifier],
    this: Vec<Modifier>,
    child_declared_font_weight: bool,
) -> Vec<Modifier> {
    let mut out = this;
    for m in parent {
        if !is_inheritable(m.as_type()) {
            continue;
        }
        if matches!(m, Modifier::Bold) && child_declared_font_weight {
            continue;
        }
        if !out.iter().any(|c| c.as_type() == m.as_type()) {
            out.push(m.clone());
        }
    }
    out
}

pub fn split_modifiers(all: Vec<Modifier>) -> (Vec<Modifier>, Vec<Modifier>) {
    let mut inline = vec![];
    let mut block = vec![];
    for m in all {
        if is_block_level(m.as_type()) {
            block.push(m);
        } else {
            inline.push(m);
        }
    }
    (inline, block)
}

pub fn merge_pending_block(pending: &[Modifier], new: Vec<Modifier>) -> Vec<Modifier> {
    let mut out = new;
    for m in pending {
        if !out.iter().any(|x| x.as_type() == m.as_type()) {
            out.push(m.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inner_wins() {
        let p = vec![Modifier::TextColor {
            value: "red".into(),
        }];
        let t = vec![Modifier::TextColor {
            value: "blue".into(),
        }];
        let m = merge_with_inheritance(&p, t, false);
        assert_eq!(m.len(), 1);
        assert!(matches!(&m[0], Modifier::TextColor { value } if value == "blue"));
    }
    #[test]
    fn parent_fills_gap() {
        let p = vec![Modifier::FontSize { value: 1600 }];
        let m = merge_with_inheritance(&p, vec![], false);
        assert!(matches!(m[0], Modifier::FontSize { value: 1600 }));
    }
    #[test]
    fn link_is_inheritable() {
        let p = vec![Modifier::Link {
            href: "https://a.com".into(),
        }];
        let m = merge_with_inheritance(&p, vec![], false);
        assert_eq!(m.len(), 1);
    }
    #[test]
    fn alignment_split_to_block() {
        let all = vec![
            Modifier::Bold,
            Modifier::Alignment {
                value: editor_model::Alignment::Center,
            },
        ];
        let (inline, block) = split_modifiers(all);
        assert_eq!(inline.len(), 1);
        assert_eq!(block.len(), 1);
    }
    #[test]
    fn child_declared_font_weight_suppresses_parent_bold() {
        let parent = vec![Modifier::Bold];
        let child = vec![Modifier::FontWeight { value: 500 }];
        let m = merge_with_inheritance(&parent, child, true);
        assert!(
            m.iter()
                .any(|x| matches!(x, Modifier::FontWeight { value: 500 }))
        );
        assert!(
            !m.iter().any(|x| matches!(x, Modifier::Bold)),
            "child's declared font-weight must suppress inherited Bold"
        );
    }

    #[test]
    fn child_declared_font_weight_suppresses_parent_bold_even_if_resolved_empty() {
        let parent = vec![Modifier::Bold];
        let child: Vec<Modifier> = vec![];
        let m = merge_with_inheritance(&parent, child, true);
        assert!(
            !m.iter().any(|x| matches!(x, Modifier::Bold)),
            "raw declaration must suppress Bold even if resolved modifier was dropped"
        );
    }

    #[test]
    fn child_without_font_weight_keeps_parent_bold() {
        let parent = vec![Modifier::Bold];
        let child = vec![Modifier::Italic];
        let m = merge_with_inheritance(&parent, child, false);
        assert!(m.iter().any(|x| matches!(x, Modifier::Bold)));
    }

    #[test]
    fn pending_inner_wins() {
        let p = vec![Modifier::Alignment {
            value: editor_model::Alignment::Left,
        }];
        let n = vec![Modifier::Alignment {
            value: editor_model::Alignment::Center,
        }];
        let m = merge_pending_block(&p, n);
        assert!(matches!(
            &m[0],
            Modifier::Alignment {
                value: editor_model::Alignment::Center
            }
        ));
    }
}
