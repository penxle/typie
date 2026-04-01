use editor_model::{Modifier, ModifierType, Node, NodeRef, NodeType};

use crate::Schema;
use crate::spec::{ModifierSpec, NodeSpec};

pub trait NodeSpecExt {
    fn spec(&self) -> &'static NodeSpec;
}

impl NodeSpecExt for NodeRef<'_> {
    fn spec(&self) -> &'static NodeSpec {
        Schema::node_spec(self.as_type())
    }
}

impl NodeSpecExt for Node {
    fn spec(&self) -> &'static NodeSpec {
        Schema::node_spec(self.as_type())
    }
}

impl NodeSpecExt for NodeType {
    fn spec(&self) -> &'static NodeSpec {
        Schema::node_spec(*self)
    }
}

pub trait ModifierSpecExt {
    fn spec(&self) -> &'static ModifierSpec;
}

impl ModifierSpecExt for Modifier {
    fn spec(&self) -> &'static ModifierSpec {
        Schema::modifier_spec(self.as_type())
    }
}

impl ModifierSpecExt for ModifierType {
    fn spec(&self) -> &'static ModifierSpec {
        Schema::modifier_spec(*self)
    }
}
