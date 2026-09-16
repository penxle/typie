import CoreText
import SwiftUI

public enum TFontFamily: Sendable {
  case ui

  func postScriptName(weight: TFontWeight) -> String {
    switch (self, weight) {
    case (.ui, .regular): "SUIT-Regular"
    case (.ui, .medium): "SUIT-Medium"
    case (.ui, .semibold): "SUIT-SemiBold"
    case (.ui, .bold): "SUIT-Bold"
    }
  }
}

public enum TFontWeight: Sendable {
  case regular
  case medium
  case semibold
  case bold
}

public struct TTextStyle: Sendable {
  public let family: TFontFamily
  public let size: CGFloat
  public let weight: TFontWeight
  public let lineHeight: CGFloat

  public init(family: TFontFamily = .ui, size: CGFloat, weight: TFontWeight, lineHeight: CGFloat) {
    self.family = family
    self.size = size
    self.weight = weight
    self.lineHeight = lineHeight
  }

  var fontName: String { family.postScriptName(weight: weight) }

  public var font: Font { .custom(fontName, fixedSize: size) }

  var naturalLineHeight: CGFloat {
    let font = CTFontCreateWithName(fontName as CFString, size, nil)
    return CTFontGetAscent(font) + CTFontGetDescent(font) + CTFontGetLeading(font)
  }

  var resolvedFontName: String {
    CTFontCopyPostScriptName(CTFontCreateWithName(fontName as CFString, size, nil)) as String
  }

  public var extraLineSpacing: CGFloat { max(0, lineHeight - naturalLineHeight) }
}

public enum TTypography {
  public static let display = TTextStyle(size: 28, weight: .semibold, lineHeight: 36)
  public static let heading = TTextStyle(size: 22, weight: .semibold, lineHeight: 28)
  public static let title = TTextStyle(size: 17, weight: .semibold, lineHeight: 22)
  public static let label = TTextStyle(size: 15, weight: .semibold, lineHeight: 20)
  public static let body = TTextStyle(size: 16, weight: .regular, lineHeight: 24)
  public static let action = TTextStyle(size: 15, weight: .medium, lineHeight: 20)
  public static let caption = TTextStyle(size: 13, weight: .regular, lineHeight: 18)
  public static let micro = TTextStyle(size: 11, weight: .regular, lineHeight: 16)
}
