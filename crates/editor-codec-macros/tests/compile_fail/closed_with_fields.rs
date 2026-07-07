use editor_codec_macros::Durable;

#[derive(Durable)]
#[durable(closed)]
pub enum E {
    #[durable(n(0))]
    A { x: u64 },
}

fn main() {}
