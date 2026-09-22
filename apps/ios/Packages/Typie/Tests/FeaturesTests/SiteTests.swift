import ApolloTestSupport
import Foundation
import GraphQL
import GraphQLMocks
import Testing

@testable import Features

@Suite struct SiteTests {
  @Test func identityComesFromId() async {
    let logo = await Img_image.from(
      Mock<GraphQLMocks.Image>(
        height: 64, id: GraphQL.ID("img-1"), url: "https://img.example.test/logo.png", width: 64))
    let site = Site(id: "site-1", name: "site one", url: "site-one", logo: logo)

    #expect(site.id == "site-1")
    #expect(site == Site(id: "site-1", name: "site one", url: "site-one", logo: logo))
    #expect(site != Site(id: "site-2", name: "site one", url: "site-one", logo: logo))
    #expect(Site(id: "site-1", name: "site one", url: "site-one", logo: nil).logo == nil)
  }
}
