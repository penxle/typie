import Design
import GraphQL
import SwiftUI

struct Img: View {
  private let image: Img_image?
  private let side: CGFloat

  init(_ image: Img_image?, side: CGFloat) {
    self.image = image
    self.side = side
  }

  nonisolated static func url(of image: Img_image?) -> URL? {
    guard let image, let url = URL(string: image.url), !image.url.isEmpty else { return nil }
    return url
  }

  var body: some View {
    TImage(url: Self.url(of: image), side: side)
  }
}

#if canImport(UIKit)

  import UIKit

  extension Img {
    nonisolated static func load(_ image: Img_image?, side: CGFloat, scale: CGFloat) async
      -> UIImage?
    {
      guard let url = url(of: image) else { return nil }
      return await TImage.load(url, side: side, scale: scale)
    }
  }

#endif
