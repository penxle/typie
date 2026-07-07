use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(frozen)]
pub struct S {
    #[durable(default)]
    pub a: u64,
}

fn main() {}
