import Design
import SwiftUI
import UIKit

enum ThemedHostingController {
  @MainActor
  static func make(title: String, _ content: some View) -> UIViewController {
    let controller = UIHostingController(rootView: content.themed())
    controller.title = title
    controller.navigationItem.backButtonDisplayMode = .minimal
    return controller
  }
}
