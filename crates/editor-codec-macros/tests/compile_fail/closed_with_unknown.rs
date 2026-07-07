use editor_codec::framing::UnknownPayload;
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(closed)]
pub enum E {
    #[durable(n(0))]
    A,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

fn main() {}
