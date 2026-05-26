use editor_model::Doc;

use super::emit;
use super::view::ProseText;

pub trait DocProseExt {
    fn prose(&self) -> ProseText;
}

impl DocProseExt for Doc {
    fn prose(&self) -> ProseText {
        emit::run(self)
    }
}
