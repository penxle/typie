import Foundation
import Testing

@testable import Core

final class CreateSiteStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var responseBody =
    #"{"data":{"createSite":{"__typename":"Site","id":"site-1"}}}"#
  nonisolated(unsafe) private static var recordedBody: Data?

  static func reset(
    body: String = #"{"data":{"createSite":{"__typename":"Site","id":"site-1"}}}"#
  ) {
    lock.withLock {
      responseBody = body
      recordedBody = nil
    }
  }

  static var lastBody: Data? { lock.withLock { recordedBody } }

  override func startLoading() {
    let body = Self.lock.withLock { () -> String in
      Self.recordedBody = Self.body(of: request)
      return Self.responseBody
    }
    respond(headers: ["Content-Type": "application/json"], body: Data(body.utf8))
  }
}

@Suite(.serialized) struct CreateSiteTests {
  private func makeCreate() throws -> CreateSite {
    CreateSite(
      client: GraphQLClient.make(
        config: try makeTestConfig(), deviceHeaders: { [:] }, accessToken: { nil },
        onSessionCookie: { _ in }, configuration: stubbedConfiguration(CreateSiteStub.self)))
  }

  @Test func sendsNameAndReturnsId() async throws {
    CreateSiteStub.reset(body: #"{"data":{"createSite":{"__typename":"Site","id":"site-9"}}}"#)
    let create = try makeCreate()
    let id = try await create(name: "space-a")
    #expect(id == "site-9")
    let body = try #require(CreateSiteStub.lastBody)
    let json = try #require(try JSONSerialization.jsonObject(with: body) as? [String: Any])
    let input = try #require((json["variables"] as? [String: Any])?["input"] as? [String: Any])
    #expect(input["name"] as? String == "space-a")
  }

  @Test func mapsTypieError() async throws {
    CreateSiteStub.reset(
      body:
        #"{"data":null,"errors":[{"message":"m","extensions":{"type":"TypieError","code":"subscription_required"}}]}"#
    )
    let create = try makeCreate()
    await #expect(throws: TypieError(code: "subscription_required", message: "m")) {
      _ = try await create(name: "x")
    }
  }

  @Test func failsWithoutData() async throws {
    CreateSiteStub.reset(body: "{}")
    let create = try makeCreate()
    await #expect(throws: HTTPError.malformedResponse("createSite: no data")) {
      _ = try await create(name: "x")
    }
  }
}
