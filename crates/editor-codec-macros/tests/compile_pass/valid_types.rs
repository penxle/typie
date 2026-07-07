use editor_codec::framing::{UnknownPayload, UnknownTail};
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(evolvable)]
pub struct Rec {
    pub a: u64,
    pub tail: UnknownTail,
}

#[derive(Durable)]
#[durable(frozen)]
pub struct Anchor {
    pub id: u64,
    pub bias: u8,
}

#[derive(Durable)]
#[durable(open)]
#[durable(retired(9))]
pub enum Item {
    #[durable(n(0))]
    #[durable(frozen)]
    Char(char),
    #[durable(n(1))]
    Block { node_type: u32, tail: UnknownTail },
    #[durable(n(2))]
    Break,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

#[derive(Durable)]
#[durable(closed)]
pub enum Bias {
    #[durable(n(0))]
    Before,
    #[durable(n(1))]
    After,
}

fn main() {}
