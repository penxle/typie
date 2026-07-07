use editor_codec_macros::Durable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Durable)]
#[durable(closed)]
pub enum DurableBias {
    #[durable(n(0))]
    Before,
    #[durable(n(1))]
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Durable)]
#[durable(frozen)]
pub struct DurableAnchor {
    pub id: editor_crdt::Dot,
    pub bias: DurableBias,
}
