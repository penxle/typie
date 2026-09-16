import SwiftUI

struct ARGB: Sendable, Equatable {
  let alpha: Double
  let red: Double
  let green: Double
  let blue: Double

  init(_ value: UInt32) {
    alpha = Double((value >> 24) & 0xFF) / 255.0
    red = Double((value >> 16) & 0xFF) / 255.0
    green = Double((value >> 8) & 0xFF) / 255.0
    blue = Double(value & 0xFF) / 255.0
  }
}

extension Color {
  init(argb value: UInt32) {
    let c = ARGB(value)
    self.init(.sRGB, red: c.red, green: c.green, blue: c.blue, opacity: c.alpha)
  }
}
