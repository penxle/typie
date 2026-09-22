import ApolloTestSupport
import Foundation
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

@Suite struct ImgTests {
  private func image(url: String) async -> Img_image {
    await Img_image.from(
      Mock<GraphQLMocks.Image>(height: 180, id: GraphQL.ID("image-1"), url: url, width: 320))
  }

  @Test func urlComesFromFragment() async throws {
    let fragment = await image(url: "https://img.example.test/a.png")
    #expect(Img.url(of: fragment) == URL(string: "https://img.example.test/a.png"))
  }

  @Test func urlIsNilForEmptyString() async {
    let fragment = await image(url: "")
    #expect(Img.url(of: fragment) == nil)
  }

  @Test func urlIsNilWithoutImage() {
    #expect(Img.url(of: nil) == nil)
  }
}
