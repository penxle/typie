import Apollo
import Foundation
import Testing

@testable import Core

final class AuthGraphQLStub: StubURLProtocol, @unchecked Sendable {
  struct Stubbed: Sendable {
    var statusCode = 200
    var headerFields: [String: String] = ["Content-Type": "application/json"]
    var body = #"{"data":{"randomName":"probe","me":null}}"#
  }

  private static let lock = NSLock()
  nonisolated(unsafe) private static var stubbed = Stubbed()
  nonisolated(unsafe) private static var recorded: URLRequest?

  static func reset(_ response: Stubbed = Stubbed()) {
    lock.withLock {
      stubbed = response
      recorded = nil
    }
  }

  static var lastRequest: URLRequest? { lock.withLock { recorded } }

  override func startLoading() {
    let response = Self.lock.withLock { () -> Stubbed in
      Self.recorded = request
      return Self.stubbed
    }
    respond(
      status: response.statusCode, headers: response.headerFields,
      body: Data(response.body.utf8))
  }
}

@Suite(.serialized) struct GraphQLAuthTests {
  private func makeClient(
    accessToken: @escaping @Sendable () -> String? = { nil },
    onSessionCookie: @escaping @Sendable (String) async throws -> Void = { _ in }
  ) throws -> GraphQLClient {
    GraphQLClient.make(
      config: try makeTestConfig(),
      deviceHeaders: { DeviceHeaders.make(deviceID: "abc", model: "iPhone", systemName: "iOS") },
      accessToken: accessToken, onSessionCookie: onSessionCookie,
      configuration: stubbedConfiguration(AuthGraphQLStub.self))
  }

  private func setCookieHeader(
    _ value: String, statusCode: Int = 200, headerName: String = "Set-Cookie"
  ) {
    AuthGraphQLStub.reset(
      AuthGraphQLStub.Stubbed(
        statusCode: statusCode,
        headerFields: ["Content-Type": "application/json", headerName: value]))
  }

  @Test func capturesSessionCookieFromLowercaseHeaderName() async throws {
    setCookieHeader("typie-st=abc; Path=/; HttpOnly", headerName: "set-cookie")
    let recorder = CallRecorder()
    let client = try makeClient(onSessionCookie: { recorder.record($0) })
    _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    #expect(recorder.calls == ["abc"])
  }

  @Test func attachesBearerHeaderWhenAuthenticated() async throws {
    AuthGraphQLStub.reset()
    let client = try makeClient(accessToken: { "token-value" })
    _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    let request = try #require(AuthGraphQLStub.lastRequest)
    #expect(request.value(forHTTPHeaderField: "Authorization") == "Bearer token-value")
    #expect(request.value(forHTTPHeaderField: "X-Device-Id") == "abc")
    #expect(request.value(forHTTPHeaderField: "X-Device-Name") == "iPhone")
    #expect(request.value(forHTTPHeaderField: "X-Device-Platform") == "IOS")
  }

  @Test func omitsBearerHeaderWhenUnauthenticated() async throws {
    AuthGraphQLStub.reset()
    let client = try makeClient()
    _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    let request = try #require(AuthGraphQLStub.lastRequest)
    #expect(request.value(forHTTPHeaderField: "Authorization") == nil)
    #expect(request.value(forHTTPHeaderField: "X-Device-Id") == "abc")
  }

  @Test func capturesSessionCookieOnce() async throws {
    setCookieHeader("typie-st=abc; Path=/; HttpOnly")
    let recorder = CallRecorder()
    let client = try makeClient(onSessionCookie: { recorder.record($0) })
    _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    #expect(recorder.calls == ["abc"])
  }

  @Test func capturesSessionCookieAmongOtherCookies() async throws {
    setCookieHeader("other=1; Path=/, typie-st=abc; Path=/; HttpOnly")
    let recorder = CallRecorder()
    let client = try makeClient(onSessionCookie: { recorder.record($0) })
    _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    #expect(recorder.calls == ["abc"])
  }

  @Test func capturesSessionCookieWhenOtherCookieHasExpiresAttribute() async throws {
    setCookieHeader(
      "other=1; Expires=Wed, 01 Jan 2030 00:00:00 GMT; Path=/, typie-st=abc; Path=/; HttpOnly")
    let recorder = CallRecorder()
    let client = try makeClient(onSessionCookie: { recorder.record($0) })
    _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    #expect(recorder.calls == ["abc"])
  }

  @Test func ignoresCookiesWithDifferentName() async throws {
    setCookieHeader("typie-st-x=abc; Path=/, typie-stx=def; Path=/")
    let recorder = CallRecorder()
    let client = try makeClient(onSessionCookie: { recorder.record($0) })
    _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    #expect(recorder.calls.isEmpty)
  }

  @Test func ignoresSessionCookieOnFailureResponse() async throws {
    setCookieHeader("typie-st=abc; Path=/; HttpOnly", statusCode: 500)
    let recorder = CallRecorder()
    let client = try makeClient(onSessionCookie: { recorder.record($0) })
    await #expect(throws: ResponseCodeInterceptor.ResponseCodeError.self) {
      _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    }
    #expect(recorder.calls.isEmpty)
  }

  @Test func failsRequestWhenSessionEstablishmentFails() async throws {
    setCookieHeader("typie-st=abc; Path=/; HttpOnly")
    let client = try makeClient(onSessionCookie: { _ in throw HTTPError.status(503) })
    do {
      _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
      Issue.record("expected an error")
    } catch let error as SessionEstablishmentError {
      #expect(error.underlying as? HTTPError == .status(503))
      #expect(error.localizedDescription == "HTTP 503")
    }
  }

  @Test func propagatesCancellationFromSessionEstablishmentUnwrapped() async throws {
    setCookieHeader("typie-st=abc; Path=/; HttpOnly")
    let client = try makeClient(onSessionCookie: { _ in throw CancellationError() })
    await #expect(throws: CancellationError.self) {
      _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    }
  }

  @Test func capturesSessionCookieCarryingItsOwnExpiresAttribute() async throws {
    setCookieHeader("typie-st=abc; Path=/; Expires=Wed, 01 Jan 2030 00:00:00 GMT; HttpOnly")
    let recorder = CallRecorder()
    let client = try makeClient(onSessionCookie: { recorder.record($0) })
    _ = try await client.apollo.fetch(query: ServerProbe_Query(), cachePolicy: .networkOnly)
    #expect(recorder.calls == ["abc"])
  }
}
