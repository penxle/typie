mod inspect_doc;
mod inspect_macro;
mod inspect_page;
mod inspect_selection;
mod inspect_state;
mod utils;

pub use inspect_macro::{inspect_fragment_as_macro, inspect_state_as_macro};
pub use inspect_page::inspect_page_element;
pub use inspect_state::inspect_state;
