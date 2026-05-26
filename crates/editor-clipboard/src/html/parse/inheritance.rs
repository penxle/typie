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

pub fn merge_with_inheritance(parent: &[Modifier], this: Vec<Modifier>) -> Vec<Modifier> {
    let mut out = this;
    for m in parent {
        if !is_inheritable(m.as_type()) {
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
        let m = merge_with_inheritance(&p, t);
        assert_eq!(m.len(), 1);
        assert!(matches!(&m[0], Modifier::TextColor { value } if value == "blue"));
    }
    #[test]
    fn parent_fills_gap() {
        let p = vec![Modifier::FontSize { value: 1600 }];
        let m = merge_with_inheritance(&p, vec![]);
        assert!(matches!(m[0], Modifier::FontSize { value: 1600 }));
    }
    #[test]
    fn link_is_inheritable() {
        let p = vec![Modifier::Link {
            href: "https://a.com".into(),
        }];
        let m = merge_with_inheritance(&p, vec![]);
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
