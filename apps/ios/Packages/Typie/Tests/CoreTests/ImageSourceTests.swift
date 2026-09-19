import Foundation
import Testing

@_spi(Unsafe) import ApolloAPI

@testable import Core

@Suite struct ImageSourceTests {
  private func makeFragment(url: String) -> TImage_image {
    TImage_image(
      unsafelyWithData: [
        "__typename": "Image", "id": "image-1", "url": url, "width": 320, "height": 180,
      ])
  }

  @Test func buildsFromFragmentWithParsableURL() throws {
    let source = try #require(ImageSource(makeFragment(url: "https://img.example.test/a.png")))
    #expect(
      source
        == ImageSource(
          url: try #require(URL(string: "https://img.example.test/a.png")), width: 320, height: 180)
    )
  }

  @Test func returnsNilForUnparsableURL() {
    #expect(ImageSource(makeFragment(url: "")) == nil)
  }
}
