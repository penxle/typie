#if canImport(UIKit)

  import Design
  import UIKit

  @MainActor
  final class SiteLogoBarItem {
    static let logoSide: CGFloat = 32

    let item: UIBarButtonItem
    let displayScale: CGFloat
    var currentURL: URL?

    private let placeholder: UIImage

    init(menu: UIMenu, displayScale: CGFloat) {
      self.displayScale = max(displayScale, 2)
      placeholder = Self.placeholder(scale: self.displayScale)
      item = UIBarButtonItem(image: placeholder, menu: menu)
      item.accessibilityLabel = "스페이스 메뉴"
    }

    func setImage(_ image: UIImage?) {
      item.image = image.map { Self.circular($0, scale: displayScale) } ?? placeholder
    }

    private static func circular(_ image: UIImage, scale: CGFloat) -> UIImage {
      let side = logoSide
      let ratio = max(side / image.size.width, side / image.size.height)
      let drawSize = CGSize(width: image.size.width * ratio, height: image.size.height * ratio)
      let origin = CGPoint(x: (side - drawSize.width) / 2, y: (side - drawSize.height) / 2)
      return render(scale: scale) { rect in
        UIBezierPath(ovalIn: rect).addClip()
        image.draw(in: CGRect(origin: origin, size: drawSize))
      }
    }

    private static func placeholder(scale: CGFloat) -> UIImage {
      let asset = UIImageAsset()
      let traits = { (style: UIUserInterfaceStyle) in
        UITraitCollection { mutable in
          mutable.userInterfaceStyle = style
          mutable.displayScale = scale
        }
      }
      for style in [UIUserInterfaceStyle.light, .dark] {
        let color = UIColor.theme(\.surfaceInset).resolvedColor(with: traits(style))
        let image = render(scale: scale) { rect in
          color.setFill()
          UIBezierPath(ovalIn: rect).fill()
        }
        asset.register(image, with: traits(style))
      }
      return asset.image(with: traits(.light))
    }

    private static func render(scale: CGFloat, _ draw: (CGRect) -> Void) -> UIImage {
      let format = UIGraphicsImageRendererFormat()
      format.scale = scale
      format.opaque = false
      let rect = CGRect(origin: .zero, size: CGSize(width: logoSide, height: logoSide))
      return UIGraphicsImageRenderer(size: rect.size, format: format)
        .image { _ in draw(rect) }
        .withRenderingMode(.alwaysOriginal)
    }
  }

#endif
