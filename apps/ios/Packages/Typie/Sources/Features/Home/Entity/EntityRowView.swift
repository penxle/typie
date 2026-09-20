#if canImport(UIKit)

  import Design
  import SwiftUI

  struct HighlightedTextView: View {
    @Environment(\.theme) private var theme

    let text: HighlightedText
    var metrics: TTextMetrics
    let color: Color
    let maxLines: Int?

    init(text: HighlightedText, style: TTextStyle, color: Color, maxLines: Int? = 1) {
      self.text = text
      metrics = TTextMetrics(style)
      self.color = color
      self.maxLines = maxLines
    }

    var body: some View {
      text.segments.reduce(Text("")) { partial, segment in
        partial + segmentText(segment)
      }
      .font(metrics.style.font)
      .lineLimit(maxLines)
      .lineSpacing(metrics.extraLineSpacing)
    }

    private func segmentText(_ segment: HighlightedText.Segment) -> Text {
      Text(segment.text).foregroundStyle(segment.isHighlighted ? theme.colors.paletteBlue : color)
    }
  }

  public struct EntityRowView: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    private let icon: EntityIconSpec
    private let path: [String]
    private let trailing: String?
    private let title: HighlightedText
    private let snippet: HighlightedText?

    public init(
      icon: EntityIconSpec, path: [String], trailing: String?, title: HighlightedText,
      snippet: HighlightedText? = nil
    ) {
      self.icon = icon
      self.path = path
      self.trailing = trailing
      self.title = title
      self.snippet = snippet
    }

    public var body: some View {
      let appearance = EntityIcon.appearance(icon, colors: colors)
      VStack(alignment: .leading, spacing: 0) {
        HStack(spacing: 8) {
          EntityPathText(segments: path)
          if let trailing {
            TText(trailing, style: TTypography.meta, color: colors.textHint, maxLines: 1)
              .layoutPriority(1)
          }
        }
        Spacer().frame(height: 4)
        HStack(spacing: 8) {
          TIcon(
            appearance.icon, size: 16, tint: appearance.tint, relativeTo: TTypography.label)
          HighlightedTextView(text: title, style: TTypography.label, color: colors.textDefault)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        if let snippet, !Self.isBlank(snippet.plain) {
          Spacer().frame(height: 6)
          HighlightedTextView(
            text: snippet, style: TTypography.detail, color: colors.textMuted, maxLines: 2)
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(.vertical, 14)
      .contentShape(Rectangle())
      .accessibilityElement(children: .combine)
    }

    private static func isBlank(_ text: String) -> Bool {
      text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
  }

#endif
