import Nuke
import NukeUI
import SwiftUI

public struct TImage: View {
  @Environment(\.theme) private var theme
  @Environment(\.displayScale) private var displayScale
  private var colors: TColors { theme.colors }

  private let url: URL?
  private let side: CGFloat

  public init(url: URL?, side: CGFloat) {
    self.url = url
    self.side = side
  }

  public var body: some View {
    LazyImage(
      request: url.map { ImageRequest(url: Self.requestURL($0, side: side, scale: displayScale)) }
    ) { state in
      if let image = state.image {
        image.resizable().scaledToFill()
      } else {
        colors.surfaceInset
      }
    }
    .frame(width: side, height: side)
    .clipped()
  }

  nonisolated public static func requestedSide(points: CGFloat, scale: CGFloat) -> Int {
    let pixels = max(1, points * scale)
    return Int(pow(2, ceil(log2(pixels))))
  }

  nonisolated public static func requestURL(_ url: URL, side: CGFloat, scale: CGFloat) -> URL {
    guard var components = URLComponents(url: url, resolvingAgainstBaseURL: false) else {
      return url
    }
    var items = components.queryItems ?? []
    items.append(URLQueryItem(name: "s", value: String(requestedSide(points: side, scale: scale))))
    items.append(URLQueryItem(name: "q", value: "75"))
    components.queryItems = items
    return components.url ?? url
  }
}

#if canImport(UIKit)

  import UIKit

  extension TImage {
    nonisolated public static func load(_ url: URL, side: CGFloat, scale: CGFloat) async -> UIImage?
    {
      try? await ImagePipeline.shared.image(for: requestURL(url, side: side, scale: scale))
    }
  }

#endif
