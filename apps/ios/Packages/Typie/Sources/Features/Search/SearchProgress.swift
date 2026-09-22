#if canImport(UIKit)

  import Design
  import SwiftUI

  struct SearchProgress: View {
    @Environment(\.theme) private var theme

    private let size: CGFloat

    init(size: CGFloat) {
      self.size = size
    }

    var body: some View {
      TSpinner(color: theme.colors.textDefault, size: size)
    }
  }

#endif
