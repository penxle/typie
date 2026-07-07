use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(open)]
pub enum E {
    #[durable(n(0))]
    A,
    #[durable(unknown)]
    Unknown { tag: u64 },
}

fn main() {}
