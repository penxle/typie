#if canImport(UIKit)

  import Design
  import SwiftUI
  import UIKit

  final class ThemedHostingController: UIHostingController<AnyView> {
    init(title: String, _ content: some View) {
      super.init(rootView: AnyView(content.themed()))
      self.title = title
      navigationItem.backButtonDisplayMode = .minimal
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }
  }

#endif
