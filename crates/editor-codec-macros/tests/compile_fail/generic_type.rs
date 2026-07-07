use editor_codec::framing::UnknownTail;
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(evolvable)]
pub struct S<T> {
    pub a: T,
    pub tail: UnknownTail,
}

fn main() {}
