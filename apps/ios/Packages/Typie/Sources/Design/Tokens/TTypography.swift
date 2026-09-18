import CoreText
import SwiftUI
import Synchronization

private let naturalLineHeights = Mutex<[String: CGFloat]>([:])

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
  public let textStyle: Font.TextStyle

  public init(
    family: TFontFamily = .ui, size: CGFloat, weight: TFontWeight, lineHeight: CGFloat,
    relativeTo textStyle: Font.TextStyle
  ) {
    self.family = family
    self.size = size
    self.weight = weight
    self.lineHeight = lineHeight
    self.textStyle = textStyle
  }

  var fontName: String { family.postScriptName(weight: weight) }

  public var font: Font { .custom(fontName, size: size, relativeTo: textStyle) }

  func naturalLineHeight(atSize size: CGFloat) -> CGFloat {
    let key = "\(fontName)@\(size)"
    if let cached = naturalLineHeights.withLock({ $0[key] }) { return cached }
    let font = CTFontCreateWithName(fontName as CFString, size, nil)
    let height = CTFontGetAscent(font) + CTFontGetDescent(font) + CTFontGetLeading(font)
    naturalLineHeights.withLock { $0[key] = height }
    return height
  }

  public func extraLineSpacing(atSize size: CGFloat, lineHeight: CGFloat) -> CGFloat {
    max(0, lineHeight - naturalLineHeight(atSize: size))
  }
}

public struct TTextMetrics: DynamicProperty {
  public let style: TTextStyle
  @ScaledMetric private var size: CGFloat
  @ScaledMetric private var lineHeight: CGFloat

  public init(_ style: TTextStyle) {
    self.style = style
    _size = ScaledMetric(wrappedValue: style.size, relativeTo: style.textStyle)
    _lineHeight = ScaledMetric(wrappedValue: style.lineHeight, relativeTo: style.textStyle)
  }

  public var extraLineSpacing: CGFloat {
    style.extraLineSpacing(atSize: size, lineHeight: lineHeight)
  }
}

public enum TTypography {
  public static let hero = TTextStyle(
    size: 28, weight: .semibold, lineHeight: 36, relativeTo: .title)
  public static let heading = TTextStyle(
    size: 22, weight: .semibold, lineHeight: 28, relativeTo: .title2)
  public static let title = TTextStyle(
    size: 17, weight: .semibold, lineHeight: 22, relativeTo: .headline)
  public static let text = TTextStyle(
    size: 16, weight: .regular, lineHeight: 24, relativeTo: .callout)
  public static let label = TTextStyle(
    size: 15, weight: .semibold, lineHeight: 20, relativeTo: .subheadline)
  public static let control = TTextStyle(
    size: 15, weight: .medium, lineHeight: 20, relativeTo: .subheadline)
  public static let detail = TTextStyle(
    size: 14, weight: .regular, lineHeight: 20, relativeTo: .subheadline)
  public static let caption = TTextStyle(
    size: 13, weight: .regular, lineHeight: 18, relativeTo: .footnote)
  public static let section = TTextStyle(
    size: 13, weight: .bold, lineHeight: 18, relativeTo: .footnote)
  public static let meta = TTextStyle(
    size: 12, weight: .regular, lineHeight: 16, relativeTo: .caption)
  public static let fine = TTextStyle(
    size: 11, weight: .regular, lineHeight: 16, relativeTo: .caption2)
}

#if canImport(UIKit)

  import UIKit

  extension Font.TextStyle {
    var uiKit: UIFont.TextStyle {
      switch self {
      case .extraLargeTitle: .extraLargeTitle
      case .extraLargeTitle2: .extraLargeTitle2
      case .largeTitle: .largeTitle
      case .title: .title1
      case .title2: .title2
      case .title3: .title3
      case .headline: .headline
      case .subheadline: .subheadline
      case .body: .body
      case .callout: .callout
      case .footnote: .footnote
      case .caption: .caption1
      case .caption2: .caption2
      @unknown default: .body
      }
    }
  }

  extension TTextStyle {
    public func scaled(for traits: UITraitCollection) -> (size: CGFloat, lineHeight: CGFloat) {
      let metrics = UIFontMetrics(forTextStyle: textStyle.uiKit)
      return (
        metrics.scaledValue(for: size, compatibleWith: traits),
        metrics.scaledValue(for: lineHeight, compatibleWith: traits)
      )
    }

    public func uiFont(atSize size: CGFloat) -> UIFont {
      UIFont(name: fontName, size: size) ?? .systemFont(ofSize: size)
    }

    public func uiFont(for traits: UITraitCollection) -> UIFont {
      UIFontMetrics(forTextStyle: textStyle.uiKit).scaledFont(
        for: uiFont(atSize: size), compatibleWith: traits)
    }
  }

#endif
