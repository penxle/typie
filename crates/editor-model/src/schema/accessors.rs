use crate::{Modifier, ModifierType, Node, NodeType};

use super::{ModifierSpec, NodeSpec, Schema};

impl NodeType {
    pub fn spec(self) -> &'static NodeSpec {
        Schema::node_spec(self)
    }
}

impl Node {
    pub fn spec(&self) -> &'static NodeSpec {
        Schema::node_spec(self.as_type())
    }
}

impl ModifierType {
    pub fn spec(self) -> &'static ModifierSpec {
        Schema::modifier_spec(self)
    }
}

impl Modifier {
    pub fn spec(&self) -> &'static ModifierSpec {
        Schema::modifier_spec(self.as_type())
    }
}
