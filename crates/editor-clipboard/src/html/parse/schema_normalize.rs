use editor_model::{
    Fragment, PlainBulletListNode, PlainNode, PlainParagraphNode, PlainTableNode, PlainTableRowNode,
};

pub fn normalize(children: Vec<Fragment>) -> Vec<Fragment> {
    let mut result: Vec<Fragment> = vec![];
    let mut inline_run: Vec<Fragment> = vec![];
    fn flush_inline(inline_run: &mut Vec<Fragment>, result: &mut Vec<Fragment>) {
        if !inline_run.is_empty() {
            result.push(Fragment {
                node: PlainNode::Paragraph(PlainParagraphNode::default()),
                modifiers: vec![],
                style: None,
                children: std::mem::take(inline_run),
            });
        }
    }
    for child in children {
        match &child.node {
            PlainNode::Text(_) | PlainNode::HardBreak(_) => inline_run.push(child),
            PlainNode::ListItem(_) => {
                flush_inline(&mut inline_run, &mut result);
                result.push(Fragment {
                    node: PlainNode::BulletList(PlainBulletListNode::default()),
                    modifiers: vec![],
                    style: None,
                    children: vec![child],
                });
            }
            PlainNode::TableRow(_) => {
                flush_inline(&mut inline_run, &mut result);
                result.push(Fragment {
                    node: PlainNode::Table(PlainTableNode::default()),
                    modifiers: vec![],
                    style: None,
                    children: vec![child],
                });
            }
            PlainNode::TableCell(_) => {
                flush_inline(&mut inline_run, &mut result);
                result.push(Fragment {
                    node: PlainNode::Table(PlainTableNode::default()),
                    modifiers: vec![],
                    style: None,
                    children: vec![Fragment {
                        node: PlainNode::TableRow(PlainTableRowNode::default()),
                        modifiers: vec![],
                        style: None,
                        children: vec![child],
                    }],
                });
            }
            _ => {
                flush_inline(&mut inline_run, &mut result);
                result.push(child);
            }
        }
    }
    flush_inline(&mut inline_run, &mut result);
    result
}
