internal import EditorFFI

public struct EditorDocument: Sendable {
  let plain: PlainDoc

  init(plain: PlainDoc) {
    self.plain = plain
  }
}

public enum EditorSyntheticLayout: Sendable, CaseIterable {
  case paginated
  case continuous
}

extension EditorDocument {
  static let syntheticParagraphs = 3000
  static let syntheticFontFamily = "Pretendard"

  public static func synthetic(_ layout: EditorSyntheticLayout) -> EditorDocument {
    var children = [
      syntheticParagraph("Typie editor spike"),
      syntheticParagraph("The quick brown fox jumps over the lazy dog. 0123456789"),
    ]
    for index in 0..<syntheticParagraphs {
      children.append(
        syntheticParagraph(
          "문단 \(index): 다람쥐 헌 쳇바퀴에 타고파. 합성 텍스트로 줄바꿈과 글리프 래스터를 확인한다. "
            + "Lorem ipsum dolor sit amet, consectetur adipiscing elit."))
    }
    return EditorDocument(
      plain: PlainDoc(
        root: PlainNodeEntry(
          node: .root(PlainRootNode(layoutMode: syntheticLayoutMode(layout))),
          modifiers: [
            .fontFamily: .fontFamily(value: syntheticFontFamily),
            .fontSize: .fontSize(value: 1200),
            .fontWeight: .fontWeight(value: 400),
            .textColor: .textColor(value: "black"),
            .backgroundColor: .backgroundColor(value: "none"),
            .letterSpacing: .letterSpacing(value: 0),
            .lineHeight: .lineHeight(value: 160),
            .paragraphIndent: .paragraphIndent(value: 100),
            .blockGap: .blockGap(value: 100),
          ],
          children: children)))
  }

  static func syntheticLayoutMode(_ layout: EditorSyntheticLayout) -> LayoutMode {
    switch layout {
    case .paginated:
      .paginated(
        pageWidth: 794, pageHeight: 1123, pageMarginTop: 94, pageMarginBottom: 94,
        pageMarginLeft: 94, pageMarginRight: 94)
    case .continuous:
      .continuous(maxWidth: 600)
    }
  }

  private static func syntheticParagraph(_ text: String) -> PlainNodeEntry {
    PlainNodeEntry(
      node: .paragraph, modifiers: [:],
      children: [
        PlainNodeEntry(node: .text(PlainTextNode(text: text)), modifiers: [:], children: [])
      ])
  }
}
