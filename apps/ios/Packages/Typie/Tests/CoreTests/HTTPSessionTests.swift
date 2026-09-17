import Alamofire
import Foundation
import Testing

@testable import Core

final class RedirectStub: StubURLProtocol, @unchecked Sendable {
  static let location = "typie:///authorize?error=login_required"

  override func startLoading() {
    guard request.url?.scheme == "https" else {
      respond()
      return
    }
    respond(status: 302, headers: ["Location": Self.location])
  }
}

@Suite struct HTTPSessionTests {
  @Test func configurationDisablesCookies() {
    let configuration = HTTPSession.configuration()
    #expect(configuration.httpCookieStorage == nil)
    #expect(configuration.httpShouldSetCookies == false)
    #expect(configuration.httpCookieAcceptPolicy == .never)
    #expect(configuration.timeoutIntervalForRequest == 60)
  }

  @Test func doesNotFollowRedirects() async throws {
    let session = HTTPSession.make(configuration: stubbedConfiguration(RedirectStub.self))
    let response = await session.request("https://auth.example.test/authorize")
      .serializingData(emptyResponseCodes: [302]).response
    #expect(response.response?.statusCode == 302)
    #expect(response.response?.value(forHTTPHeaderField: "Location") == RedirectStub.location)
  }
}
