use editor_codec::framing::UnknownTail;
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(evolvable)]
pub struct S {
    pub a: u64,
    #[durable(default)]
    pub tail: UnknownTail,
}

fn main() {}
