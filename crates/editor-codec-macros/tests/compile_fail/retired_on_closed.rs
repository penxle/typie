use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(closed)]
#[durable(retired(1))]
pub enum E {
    #[durable(n(0))]
    A,
}

fn main() {}
