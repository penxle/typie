import CoreText
import Foundation

public enum TFonts {
  private static let registration: Void = {
    for url in Bundle.module.urls(forResourcesWithExtension: "ttf", subdirectory: nil) ?? [] {
      CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
    }
  }()

  public static func registerAll() {
    registration
  }
}
