import Apollo
import Foundation
import Testing

@testable import Core

final class GraphQLStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var recorded: URLRequest?

  static var lastRequest: URLRequest? {
    lock.withLock { recorded }
  }

  override func startLoading() {
    if request.url?.host() == "redirect.example.test" {
      respond(status: 302, headers: ["Location": "https://api.example.test/followed"])
      return
    }
    Self.lock.withLock { Self.recorded = request }
    respond(
      headers: ["Content-Type": "application/json"],
      body: Data(#"{"data":{"randomName":"probe","me":null}}"#.utf8))
  }
}

@Suite(.serialized) struct GraphQLClientTests {
  @Test func sendsDeviceHeadersToGraphQLEndpoint() async throws {
    let client = GraphQLClient.make(
      config: try makeTestConfig(),
      deviceHeaders: { DeviceHeaders.make(deviceID: "abc", model: "iPhone", systemName: "iOS") },
      accessToken: { nil }, onSessionCookie: { _ in },
      configuration: stubbedConfiguration(GraphQLStub.self))
    let response = try await client.apollo.fetch(
      query: ServerProbeQuery(), cachePolicy: .networkOnly)
    #expect(response.data?.randomName == "probe")
    let request = try #require(GraphQLStub.lastRequest)
    #expect(request.url?.absoluteString == "https://api.example.test/graphql")
    #expect(request.value(forHTTPHeaderField: "X-Device-Id") == "abc")
    #expect(request.value(forHTTPHeaderField: "X-Device-Name") == "iPhone")
    #expect(request.value(forHTTPHeaderField: "X-Device-Platform") == "IOS")
  }

  @Test func redirectBlockerStopsRedirects() async throws {
    let url = URL(string: "https://redirect.example.test/start")!
    let blocked = URLSession(
      configuration: stubbedConfiguration(GraphQLStub.self), delegate: RedirectBlocker(),
      delegateQueue: nil)
    let (_, blockedResponse) = try await blocked.data(from: url)
    #expect((blockedResponse as? HTTPURLResponse)?.statusCode == 302)

    let following = URLSession(configuration: stubbedConfiguration(GraphQLStub.self))
    let (_, followedResponse) = try await following.data(from: url)
    #expect((followedResponse as? HTTPURLResponse)?.statusCode == 200)
    #expect(followedResponse.url?.path() == "/followed")
  }
}
