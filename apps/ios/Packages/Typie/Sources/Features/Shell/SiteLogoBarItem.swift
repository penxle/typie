#if canImport(UIKit)

  import Design
  import UIKit

  @MainActor
  final class SiteLogoBarItem {
    static let side: CGFloat = 28

    let item: UIBarButtonItem
    var currentURL: URL?

    private let container = UIView(frame: CGRect(x: 0, y: 0, width: side, height: side))
    private let button = UIButton(type: .custom)

    init(menu: UIMenu) {
      button.menu = menu
      button.showsMenuAsPrimaryAction = true
      button.backgroundColor = .theme(\.surfaceInset)
      button.layer.cornerRadius = TShapes.sm
      button.clipsToBounds = true
      button.contentHorizontalAlignment = .fill
      button.contentVerticalAlignment = .fill
      button.imageView?.contentMode = .scaleAspectFill
      button.translatesAutoresizingMaskIntoConstraints = false
      container.addSubview(button)
      NSLayoutConstraint.activate([
        button.widthAnchor.constraint(equalToConstant: Self.side),
        button.heightAnchor.constraint(equalToConstant: Self.side),
        button.leadingAnchor.constraint(equalTo: container.leadingAnchor),
        button.centerYAnchor.constraint(equalTo: container.centerYAnchor),
      ])
      item = UIBarButtonItem(customView: container)
      if #available(iOS 26.0, *) {
        item.hidesSharedBackground = true
      }
    }

    var displayScale: CGFloat {
      max(button.traitCollection.displayScale, 2)
    }

    func setImage(_ image: UIImage?) {
      button.setImage(image, for: .normal)
    }
  }

#endif
