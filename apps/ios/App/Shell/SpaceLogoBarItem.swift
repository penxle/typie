import Design
import SwiftUI
import UIKit

@MainActor
final class SpaceLogoBarItem {
  static let side: CGFloat = 28

  let item: UIBarButtonItem
  var currentURL: URL?

  private static let shadowOffset: CGFloat = 2
  private static let shadowBlur: CGFloat = 5
  private static let shadowAlpha: CGFloat = 0.22
  private static let margin: CGFloat = 7
  private static let leadingMargin: CGFloat = 2
  private static let width = leadingMargin + side + margin
  private static let height = side + margin * 2

  private var image: UIImage?

  init(menu: UIMenu) {
    item = UIBarButtonItem(title: nil, image: nil, primaryAction: nil, menu: menu)
    if #available(iOS 26.0, *) {
      item.hidesSharedBackground = true
    }
    render()
  }

  var displayScale: CGFloat {
    max(UITraitCollection.current.displayScale, 2)
  }

  func setImage(_ image: UIImage?) {
    self.image = image
    render()
  }

  private func render() {
    let scale = displayScale
    let asset = UIImageAsset()
    for style in [UIUserInterfaceStyle.light, .dark] {
      let traits = UITraitCollection { mutable in
        mutable.userInterfaceStyle = style
        mutable.displayScale = scale
      }
      asset.register(Self.rendered(image, traits: traits, scale: scale), with: traits)
    }
    item.image = asset.image(
      with: UITraitCollection.current.modifyingTraits { $0.displayScale = scale })
  }

  private static func rendered(_ source: UIImage?, traits: UITraitCollection, scale: CGFloat)
    -> UIImage
  {
    let format = UIGraphicsImageRendererFormat.default()
    format.scale = scale
    let rect = CGRect(x: 0, y: 0, width: side, height: side)
    let clip = TShapes.squircle(side * 0.3).path(in: rect).cgPath
    let placeholder = UIColor.theme(\.surfaceInset).resolvedColor(with: traits)
    return UIGraphicsImageRenderer(size: CGSize(width: width, height: height), format: format)
      .image { context in
        let cg = context.cgContext
        cg.translateBy(x: leadingMargin + side / 2, y: height / 2)
        cg.rotate(by: -.pi / 30)
        cg.translateBy(x: -side / 2, y: -side / 2)
        cg.saveGState()
        cg.setShadow(
          offset: CGSize(width: 0, height: shadowOffset), blur: shadowBlur,
          color: UIColor.black.withAlphaComponent(shadowAlpha).cgColor)
        cg.addPath(clip)
        cg.setFillColor(UIColor.black.cgColor)
        cg.fillPath()
        cg.restoreGState()
        cg.addPath(clip)
        cg.clip()
        if let source {
          source.draw(in: rect)
        } else {
          cg.setFillColor(placeholder.cgColor)
          cg.fill(rect)
        }
      }
      .withRenderingMode(.alwaysOriginal)
  }
}
