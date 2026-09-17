import SwiftUI

public enum TBrandMark: String, CaseIterable, Sendable {
  case google
  case kakao
  case naver
  case apple

  var assetName: String { rawValue }

  public var isMulticolor: Bool { self == .google }

  public var background: Color {
    switch self {
    case .google: .white
    case .kakao: Color(.sRGB, red: 254 / 255, green: 229 / 255, blue: 0)
    case .naver: Color(.sRGB, red: 3 / 255, green: 199 / 255, blue: 90 / 255)
    case .apple: .black
    }
  }

  public var foreground: Color {
    switch self {
    case .google, .kakao: .black
    case .naver, .apple: .white
    }
  }
}

public struct TBrandMarkView: View {
  private let mark: TBrandMark
  private let size: CGFloat

  public init(_ mark: TBrandMark, size: CGFloat = 20) {
    self.mark = mark
    self.size = size
  }

  public var body: some View {
    Image(decorative: mark.assetName, bundle: .module)
      .renderingMode(mark.isMulticolor ? .original : .template)
      .resizable()
      .aspectRatio(contentMode: .fit)
      .frame(width: size, height: size)
      .foregroundStyle(mark.foreground)
  }
}
