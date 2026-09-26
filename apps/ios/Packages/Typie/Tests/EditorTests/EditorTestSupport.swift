import EditorFFI

let paginatedLayout = LayoutMode.paginated(
  pageWidth: 794, pageHeight: 1123, pageMarginTop: 94, pageMarginBottom: 94,
  pageMarginLeft: 94, pageMarginRight: 94)

let testViewport = Viewport(width: 390, height: 844, scaleFactor: 3)

func testDocument(_ children: [PlainNodeEntry]) -> PlainDoc {
  PlainDoc(
    root: PlainNodeEntry(
      node: .root(PlainRootNode(layoutMode: paginatedLayout)), modifiers: [:], children: children))
}

func testParagraph(
  _ text: String, modifiers: [ModifierType: Modifier] = [:], carry: [Modifier] = []
) -> PlainNodeEntry {
  PlainNodeEntry(
    node: .paragraph, modifiers: [:], carry: carry,
    children: [
      PlainNodeEntry(node: .text(PlainTextNode(text: text)), modifiers: modifiers, children: [])
    ])
}

let horizontalRuleEntry = PlainNodeEntry(
  node: .horizontalRule(PlainHorizontalRuleNode()), modifiers: [:], children: [])
