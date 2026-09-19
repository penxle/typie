import Foundation
import Testing

@testable import Home

@Suite struct SpaceTests {
  @Test func identityComesFromId() throws {
    let logo = try #require(URL(string: "https://img.example.test/logo.png"))
    let space = Space(id: "space-1", name: "space one", url: "space-one", logo: logo)

    #expect(space.id == "space-1")
    #expect(space == Space(id: "space-1", name: "space one", url: "space-one", logo: logo))
    #expect(space != Space(id: "space-2", name: "space one", url: "space-one", logo: logo))
    #expect(Space(id: "space-1", name: "space one", url: "space-one", logo: nil).logo == nil)
  }
}
