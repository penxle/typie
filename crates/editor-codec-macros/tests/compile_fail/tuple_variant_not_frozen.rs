use editor_codec::framing::UnknownPayload;
use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(open)]
pub enum E {
    #[durable(n(0))]
    A(u64),
    #[durable(unknown)]
    Unknown(UnknownPayload),
}

fn main() {}
