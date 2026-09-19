#if canImport(UIKit)

  import UIKit

  struct CreateMenuItem {
    let title: String
    let image: UIImage?
    let perform: @MainActor (UIViewController) -> Void
  }

  struct CreateAction {
    enum Kind {
      case perform(@MainActor (UIViewController) -> Void)
      case menu([CreateMenuItem])
    }

    let label: String
    let image: UIImage?
    let kind: Kind
  }

#endif
