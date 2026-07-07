use editor_codec_macros::Durable;

#[derive(Durable)]
pub struct S {
    pub a: u64,
}

fn main() {}
