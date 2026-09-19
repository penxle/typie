import Foundation
import Testing

@testable import Design

@Suite struct TImageTests {
  @Test func requestedSideIsPowerOfTwoCeiling() {
    #expect(TImage.requestedSide(points: 36, scale: 3) == 128)
    #expect(TImage.requestedSide(points: 28, scale: 2) == 64)
    #expect(TImage.requestedSide(points: 64, scale: 2) == 128)
  }

  @Test func requestURLAppendsSizeAndQuality() throws {
    let base = try #require(URL(string: "https://img.example.test/a.png"))
    let url = TImage.requestURL(base, side: 36, scale: 3)
    #expect(url.absoluteString == "https://img.example.test/a.png?s=128&q=75")
  }

  @Test func requestURLKeepsExistingQuery() throws {
    let base = try #require(URL(string: "https://img.example.test/a.png?v=2"))
    let url = TImage.requestURL(base, side: 10, scale: 1)
    #expect(url.absoluteString == "https://img.example.test/a.png?v=2&s=16&q=75")
  }
}
