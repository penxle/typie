use editor_codec::framing::UnknownPayload;
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(open)]
pub enum E {
    #[durable(n(0))]
    A,
    #[durable(unknown)]
    Unknown(UnknownPayload),
    #[durable(unknown)]
    Unknown2(UnknownPayload),
}

fn main() {}
