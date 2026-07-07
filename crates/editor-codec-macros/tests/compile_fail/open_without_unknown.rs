use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(open)]
pub enum E {
    #[durable(n(0))]
    A,
}

fn main() {}
