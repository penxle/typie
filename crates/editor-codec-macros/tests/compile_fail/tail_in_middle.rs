use editor_codec::framing::UnknownTail;
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(evolvable)]
pub struct S {
    pub a: u64,
    pub tail: UnknownTail,
    pub b: u64,
}

fn main() {}
