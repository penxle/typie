use editor_codec::framing::UnknownPayload;
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(open)]
pub enum E {
    #[durable(n(0))]
    #[durable(n(1))]
    A,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

fn main() {}
