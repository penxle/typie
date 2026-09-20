import Foundation
import Testing

@testable import Features

@Suite struct SiteTests {
  @Test func identityComesFromId() throws {
    let logo = try #require(URL(string: "https://img.example.test/logo.png"))
    let site = Site(id: "site-1", name: "site one", url: "site-one", logo: logo)

    #expect(site.id == "site-1")
    #expect(site == Site(id: "site-1", name: "site one", url: "site-one", logo: logo))
    #expect(site != Site(id: "site-2", name: "site one", url: "site-one", logo: logo))
    #expect(Site(id: "site-1", name: "site one", url: "site-one", logo: nil).logo == nil)
  }
}
