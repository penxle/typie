use editor_codec::framing::UnknownPayload;
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(open)]
pub enum E {
    A,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

fn main() {}
