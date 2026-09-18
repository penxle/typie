import Design
import SwiftUI
import UIKit

enum ProfileAvatar {
  private static let side: CGFloat = 28
  private static let shadowOffset: CGFloat = 2
  private static let shadowBlur: CGFloat = 5
  private static let shadowAlpha: CGFloat = 0.22
  private static let margin: CGFloat = 7
  private static let leadingMargin: CGFloat = 2
  private static let width = leadingMargin + side + margin
  private static let height = side + margin * 2

  @MainActor
  static func barButtonItem() -> UIBarButtonItem {
    let container = UIView(frame: CGRect(x: 0, y: 0, width: width, height: height))
    let imageView = UIImageView(image: image())
    imageView.frame = container.bounds
    container.addSubview(imageView)
    let item = UIBarButtonItem(customView: container)
    if #available(iOS 26.0, *) {
      item.hidesSharedBackground = true
    }
    return item
  }

  @MainActor
  private static func image() -> UIImage? {
    guard let source = UIImage(named: "ProfilePlaceholder") else { return nil }
    let format = UIGraphicsImageRendererFormat.default()
    format.scale = 0
    let rect = CGRect(x: 0, y: 0, width: side, height: side)
    let clip = TShapes.squircle(side * 0.3).path(in: rect).cgPath
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
        source.draw(in: rect)
      }
      .withRenderingMode(.alwaysOriginal)
  }
}
