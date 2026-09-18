#if canImport(UIKit)

  import SwiftUI

  public struct TWrapLayout: Layout {
    private let spacing: CGFloat

    public init(spacing: CGFloat = 8) {
      self.spacing = spacing
    }

    public func sizeThatFits(
      proposal: ProposedViewSize, subviews: Subviews, cache: inout ()
    ) -> CGSize {
      let width = proposal.width ?? .infinity
      var x: CGFloat = 0
      var y: CGFloat = 0
      var rowHeight: CGFloat = 0
      for subview in subviews {
        var size = subview.sizeThatFits(.unspecified)
        size.width = min(size.width, width)
        if x > 0, x + size.width > width {
          x = 0
          y += rowHeight + spacing
          rowHeight = 0
        }
        x += size.width + spacing
        rowHeight = max(rowHeight, size.height)
      }
      return CGSize(width: proposal.width ?? x, height: y + rowHeight)
    }

    public func placeSubviews(
      in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()
    ) {
      var x = bounds.minX
      var y = bounds.minY
      var rowHeight: CGFloat = 0
      for subview in subviews {
        var size = subview.sizeThatFits(.unspecified)
        size.width = min(size.width, bounds.width)
        if x > bounds.minX, x + size.width > bounds.maxX {
          x = bounds.minX
          y += rowHeight + spacing
          rowHeight = 0
        }
        subview.place(
          at: CGPoint(x: x, y: y), proposal: ProposedViewSize(width: size.width, height: nil))
        x += size.width + spacing
        rowHeight = max(rowHeight, size.height)
      }
    }
  }

#endif
