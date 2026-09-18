public enum EntityPath {
  public static let root = "작업실"
  static let separator = " › "
  static let ellipsis = "…"

  public static func candidates(_ segments: [String]) -> [String] {
    guard !segments.isEmpty else { return [root] }
    let all = [root] + segments
    return (0..<all.count).map { drop in
      let shown = all.dropFirst(drop)
      return (drop > 0 ? ellipsis + separator : "") + shown.joined(separator: separator)
    }
  }

  public static func choose(_ segments: [String], fits: (String) -> Bool) -> String {
    let options = candidates(segments)
    return options.first(where: fits) ?? options[options.count - 1]
  }
}

#if canImport(UIKit)

  import Design
  import SwiftUI
  import UIKit

  struct EntityPathText: View {
    @Environment(\.theme) private var theme
    @ScaledMetric(relativeTo: TTypography.meta.textStyle) private var fontSize: CGFloat =
      TTypography.meta.size

    let segments: [String]
    @State private var width: CGFloat = .greatestFiniteMagnitude

    var body: some View {
      let font = TTypography.meta.uiFont(atSize: fontSize)
      let chosen = EntityPath.choose(segments) { candidate in
        width > 0 && (candidate as NSString).size(withAttributes: [.font: font]).width <= width
      }
      TText(chosen, style: TTypography.meta, color: theme.colors.textHint, maxLines: 1)
        .truncationMode(.tail)
        .frame(maxWidth: .infinity, alignment: .leading)
        .onGeometryChange(for: CGFloat.self) {
          $0.size.width
        } action: {
          width = $0
        }
    }
  }

#endif
