public struct HighlightedText: Equatable, Sendable {
  public struct Segment: Equatable, Sendable {
    public let text: String
    public let isHighlighted: Bool
  }

  public let segments: [Segment]

  public var plain: String { segments.map(\.text).joined() }

  public init(plain: String) {
    segments = plain.isEmpty ? [] : [Segment(text: plain, isHighlighted: false)]
  }

  public init(markup: String) {
    var segments: [Segment] = []
    var remaining = Substring(markup)
    while !remaining.isEmpty {
      guard let open = remaining.range(of: "<em>") else {
        Self.append(remaining, highlighted: false, to: &segments)
        break
      }
      Self.append(remaining[..<open.lowerBound], highlighted: false, to: &segments)
      let afterOpen = remaining[open.upperBound...]
      guard let close = afterOpen.range(of: "</em>") else {
        Self.append(remaining[open.lowerBound...], highlighted: false, to: &segments)
        break
      }
      Self.append(afterOpen[..<close.lowerBound], highlighted: true, to: &segments)
      remaining = afterOpen[close.upperBound...]
    }
    self.segments = segments
  }

  private static func append(_ raw: Substring, highlighted: Bool, to segments: inout [Segment]) {
    let text = decodeEntities(String(raw))
    guard !text.isEmpty else { return }
    segments.append(Segment(text: text, isHighlighted: highlighted))
  }

  private static func decodeEntities(_ text: String) -> String {
    text
      .replacingOccurrences(of: "&lt;", with: "<")
      .replacingOccurrences(of: "&gt;", with: ">")
      .replacingOccurrences(of: "&nbsp;", with: "\u{00A0}")
      .replacingOccurrences(of: "&amp;", with: "&")
  }
}
