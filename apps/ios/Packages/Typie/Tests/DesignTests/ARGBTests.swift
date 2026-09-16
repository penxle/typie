import Testing

@testable import Design

@Suite struct ARGBTests {
  @Test func decodesOpaqueColor() {
    let c = ARGB(0xFFD32055)
    #expect(c.alpha == 1)
    #expect(c.red == 0xD3 / 255.0)
    #expect(c.green == 0x20 / 255.0)
    #expect(c.blue == 0x55 / 255.0)
  }

  @Test func decodesAlpha() {
    let c = ARGB(0x521A180E)
    #expect(c.alpha == 0x52 / 255.0)
    #expect(c.red == 0x1A / 255.0)
  }
}
