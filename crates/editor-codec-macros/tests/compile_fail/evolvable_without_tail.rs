use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(evolvable)]
pub struct S {
    pub a: u64,
}

fn main() {}
