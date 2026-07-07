use editor_codec::framing::UnknownPayload;
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(open)]
#[durable(retired(4))]
pub enum E {
    #[durable(n(4))]
    A,
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

fn main() {}
