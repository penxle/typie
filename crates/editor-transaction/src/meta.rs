#[derive(Clone, Debug)]
pub enum HistoryTag {
    AutoReplacement,
    PasteHtml { plain_text: String },
}

#[derive(Clone, Debug, Default)]
pub enum HistoryMeta {
    #[default]
    Record,
    Tagged(HistoryTag),
    Skip,
}

#[derive(Clone, Debug, Default)]
pub struct TransactionMeta {
    pub history: HistoryMeta,
}
