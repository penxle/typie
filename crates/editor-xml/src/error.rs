use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Pos {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum XmlErrorDetail {
    #[error("xml declaration or processing instruction is not allowed")]
    Declaration,
    #[error("comment, cdata section or dtd is not allowed")]
    CommentOrDtd,
    #[error("closing tag </{name}> has no matching open element")]
    CloseWithoutOpen { name: String, open: Option<String> },
    #[error("closing tag </{name}> is not terminated")]
    CloseTagUnterminated { name: String },
    #[error("`/` must be followed by `>`")]
    SelfCloseUnterminated,
    #[error("attribute `{attr}` needs `=\"value\"`")]
    AttrMissingEquals { attr: String },
    #[error("attribute `{attr}` value must be quoted")]
    AttrUnquoted { attr: String },
    #[error("attribute `{attr}` appears twice")]
    AttrDuplicate { attr: String },
    #[error("illegal character inside a tag")]
    IllegalCharInTag,
    #[error("tag <{name} is not terminated")]
    TagUnterminated { name: String },
    #[error("a name was expected")]
    NameExpected,
    #[error("attribute value quote is not closed")]
    UnterminatedQuote,
    #[error("`<` inside an attribute value must be `&lt;`")]
    LtInAttrValue,
    #[error("forbidden control character U+{codepoint:04X}")]
    ForbiddenControlChar { codepoint: u32 },
    #[error("unknown entity; only lt gt amp quot apos and numeric references are allowed")]
    UnknownEntity,
    #[error("bad numeric character reference")]
    BadNumericReference,
    #[error("element <{name}> is not closed")]
    ElementUnclosed { name: String },

    #[error("file must start with a single <root>")]
    RootMissing,
    #[error("file root must be <root>, got <{name}>")]
    RootNotRoot { name: String },
    #[error("content after </root>")]
    TrailingContent,
    #[error("<root> may appear only once")]
    MultipleRoots,
    #[error("unknown element <{name}>")]
    UnknownElement { name: String, hint: Option<String> },
    #[error("attribute `{attr}` is not allowed on <{element}>")]
    UnknownAttribute { element: String, attr: String },
    #[error("`base` is allowed only on <root>")]
    BaseOnNonRoot,
    #[error("unknown modifier `{prefix}:{name}`")]
    UnknownModifier { prefix: String, name: String },
    #[error("`carry:{name}` is not a carry-capable modifier")]
    ModifierNotCarryKind { name: String },
    #[error("`carry:*` is allowed only on text blocks, not <{element}>")]
    CarryOnNonTextblock { element: String },
    #[error("<{element}> requires `attr:{field}`")]
    NodeAttrMissing { element: String, field: String },
    #[error("<{element}> has no attribute `attr:{field}`")]
    NodeAttrUnknown { element: String, field: String },
    #[error("`attr:{field}` on <{element}> must be a non-negative integer")]
    NodeAttrNotUnsignedInteger { element: String, field: String },
    #[error("`attr:layout_mode` must be continuous or paginated, got `{value}`")]
    LayoutModeInvalid { value: String },
    #[error("`{value}` is not an integer")]
    ValueNotInteger { value: String },
    #[error("`{modifier}` value `{value}` is out of range")]
    ValueOutOfRange { modifier: String, value: String },
    #[error("`{value}` is not an allowed value")]
    EnumValueUnknown { value: String },
    #[error("<{element}> does not take attribute `{attr}`")]
    AtomAttrNotAllowed { element: String, attr: String },
    #[error("<{element}> must be empty; write <{element}/>")]
    AtomHasContent { element: String },
    #[error("<{element}> does not take attribute `{attr}`")]
    InlineModifierAttrNotAllowed { element: String, attr: String },
    #[error("<{element}> requires `{attr}=\"…\"`")]
    InlineModifierAttrMissing { element: String, attr: String },

    #[error("character data inside <{element}> must be wrapped in <paragraph>")]
    TextInContainer { element: String },
    #[error("<{child}> cannot be placed inside <{parent}>; only text and inline modifiers")]
    BlockInsideTextblock { parent: String, child: String },
    #[error("<{parent}> content rule violated: allowed {allowed:?}, got {got:?}")]
    ContentRule {
        parent: String,
        allowed: Vec<String>,
        got: Vec<String>,
        rule: String,
    },
    #[error("<{element}> is not allowed at this position")]
    ContextNotAllowed { element: String },
    #[error(
        "the last paragraph of the document cannot end with a page break; add a paragraph after it"
    )]
    TrailingPageBreak,
    #[error("every table row needs the same number of cells; this row has {got} of {expected}")]
    TableNotRectangular { expected: usize, got: usize },
    #[error("`mod:{modifier}` is not allowed on <{element}>")]
    BlockModifierNotAllowed { modifier: String, element: String },
    #[error("<{modifier}> is not allowed around <{leaf}>")]
    InlineModifierNotAllowed { modifier: String, leaf: String },

    #[error("newline character inside text; use <hard_break/>")]
    NewlineInText,
    #[error("tab character inside text; use <tab/>")]
    TabInText,
    #[error("document contains a character that xml cannot carry: U+{codepoint:04X}")]
    ForbiddenCharInDocument { codepoint: u32 },

    #[error("`dot=\"{value}\"` is not a valid dot")]
    DotInvalid { value: String },
    #[error("dot {dot} appears twice")]
    DotDuplicate { dot: String },
    #[error("dot {dot} is not in this document")]
    DotNotInDocument { dot: String },
    #[error("dot {dot} cannot become <{new_type}>; its content does not fit")]
    DotTypeIncompatible { dot: String, new_type: String },
    #[error("<root> dot does not match this document")]
    RootDotMismatch,

    #[error("<{element}> cannot be created; it needs an existing dot")]
    OpaqueNeedsDot { element: String },
    #[error("<{element}> cannot have children")]
    OpaqueHasChildren { element: String },
    #[error("`attr:id` of <{element}> (dot {dot}) cannot change")]
    OpaqueIdChanged { element: String, dot: String },

    #[error("`base` attribute is missing")]
    BaseMissing,
    #[error("`base` attribute cannot be decoded")]
    BaseUndecodable,
    #[error("`base` heads are not in this document's history")]
    BaseNotInHistory,

    #[error("`{value}` is not an address; use a dot, an ordinal path like 3.1.2, or root")]
    AddressInvalid { value: String },
    #[error("no block at `{value}`")]
    AddressUnresolved { value: String },
    #[error("<root> cannot be deleted, replaced, moved or set")]
    RootNotEditable,
    #[error("nothing can be placed before or after <root>")]
    RootHasNoSiblings,
    #[error("<{element}> cannot hold block children")]
    TargetNotContainer { element: String },
    #[error("{target} cannot be moved into itself")]
    MoveIntoSelf { target: String },
    #[error("{inner} is inside {outer}; give only the outer one")]
    TargetsNested { outer: String, inner: String },
    #[error("replace takes exactly one element, got {count}")]
    FragmentNotSingle { count: usize },
    #[error("`{key}` cannot be set; only attr:, mod: and carry: keys can")]
    SetKeyUnknown { key: String },
    #[error("fragment has no block element")]
    FragmentEmpty,
    #[error("fragment must start with a block element")]
    FragmentNotBlock,

    #[error("document projection is degraded; refusing to edit through xml")]
    ProjectionDegraded,
    #[error("internal: {message}")]
    Internal { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct XmlError {
    pub pos: Option<Pos>,
    pub dot: Option<String>,
    /// Boxed so that every `Result` in the crate stays small: the widest detail
    /// variant is four owned strings, and it would otherwise ride on every
    /// successful return as well.
    pub detail: Box<XmlErrorDetail>,
    pub message: String,
}

impl XmlError {
    pub fn new(detail: XmlErrorDetail) -> Self {
        Self {
            pos: None,
            dot: None,
            message: detail.to_string(),
            detail: Box::new(detail),
        }
    }

    pub fn at(pos: Pos, detail: XmlErrorDetail) -> Self {
        Self {
            pos: Some(pos),
            dot: None,
            message: format!("line {} col {}: {detail}", pos.line, pos.column),
            detail: Box::new(detail),
        }
    }

    pub fn with_dot(mut self, dot: editor_crdt::Dot) -> Self {
        self.dot = Some(dot.to_string());
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(XmlErrorDetail::Internal {
            message: message.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positioned_error_prefixes_message() {
        let err = XmlError::at(Pos { line: 3, column: 7 }, XmlErrorDetail::NewlineInText);
        assert_eq!(
            err.message,
            "line 3 col 7: newline character inside text; use <hard_break/>"
        );
        assert_eq!(err.pos, Some(Pos { line: 3, column: 7 }));
        assert_eq!(err.dot, None);
        assert_eq!(*err.detail, XmlErrorDetail::NewlineInText);
    }
}
