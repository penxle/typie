import Alamofire
import Foundation
import Testing

@testable import Core

final class OIDCStub: StubURLProtocol, @unchecked Sendable {
  struct Stub {
    var statusCode: Int = 200
    var headers: [String: String] = [:]
    var body: Data?
    var failure: URLError.Code?
  }

  private static let lock = NSLock()
  nonisolated(unsafe) private static var stubs: [String: Stub] = [:]
  nonisolated(unsafe) private static var requests: [String: URLRequest] = [:]
  nonisolated(unsafe) private static var bodies: [String: Data] = [:]

  static func reset() {
    lock.withLock {
      stubs = [:]
      requests = [:]
      bodies = [:]
    }
  }

  static func set(_ path: String, _ stub: Stub) {
    lock.withLock { stubs[path] = stub }
  }

  static func recordedRequest(_ path: String) -> URLRequest? {
    lock.withLock { requests[path] }
  }

  static func recordedBody(_ path: String) -> Data? {
    lock.withLock { bodies[path] }
  }

  override func startLoading() {
    let path = request.url!.path
    let body = Self.body(of: request)
    let stub = Self.lock.withLock { () -> Stub? in
      Self.requests[path] = request
      if let body { Self.bodies[path] = body }
      return Self.stubs[path]
    }
    guard let stub else {
      client?.urlProtocol(self, didFailWithError: URLError(.unsupportedURL))
      return
    }
    if let failure = stub.failure {
      client?.urlProtocol(self, didFailWithError: URLError(failure))
      return
    }
    respond(status: stub.statusCode, headers: stub.headers, body: stub.body)
  }
}

@Suite(.serialized) struct OIDCClientTests {
  private func makeClient() throws -> OIDCClient {
    OIDCClient(
      config: makeTestConfig(),
      session: HTTPSession.make(configuration: stubbedConfiguration(OIDCStub.self)))
  }

  private func queryItems(_ request: URLRequest?) -> [String: String] {
    guard let url = request?.url,
      let components = URLComponents(url: url, resolvingAgainstBaseURL: false)
    else { return [:] }
    return (components.queryItems ?? []).reduce(into: [:]) { $0[$1.name] = $1.value ?? "" }
  }

  private func formItems(_ data: Data?) -> [String: String] {
    guard let data, let text = String(data: data, encoding: .utf8) else { return [:] }
    return text.split(separator: "&").reduce(into: [:]) { result, pair in
      let parts = pair.split(separator: "=", maxSplits: 1).map {
        String($0).replacingOccurrences(of: "+", with: " ").removingPercentEncoding ?? String($0)
      }
      guard let name = parts.first else { return }
      result[name] = parts.count > 1 ? parts[1] : ""
    }
  }

  private func capturedError(_ body: () async throws -> Void) async -> (any Error)? {
    do {
      try await body()
      return nil
    } catch {
      return error
    }
  }

  @Test func exchangeSendsAuthorizeThenToken() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/authorize",
      .init(statusCode: 302, headers: ["Location": "typie:///authorize?code=auth-code"]))
    OIDCStub.set(
      "/token",
      .init(
        statusCode: 200, headers: ["Content-Type": "application/json"],
        body: Data(#"{"access_token":"access"}"#.utf8)))

    let accessToken = try await makeClient().exchange(sessionToken: "session")
    #expect(accessToken == "access")

    let authorize = try #require(OIDCStub.recordedRequest("/authorize"))
    #expect(authorize.httpMethod == "GET")
    #expect(authorize.url?.path == "/authorize")
    #expect(authorize.url?.host == "auth.example.test")
    let query = queryItems(authorize)
    #expect(query["response_type"] == "code")
    #expect(query["redirect_uri"] == "typie:///authorize")
    #expect(query["client_id"] == "client")
    #expect(query["prompt"] == "none")
    #expect(query.count == 4)
    #expect(authorize.value(forHTTPHeaderField: "Cookie") == "typie-st=session")

    let token = try #require(OIDCStub.recordedRequest("/token"))
    #expect(token.httpMethod == "POST")
    #expect(token.url?.path == "/token")
    #expect(
      token.value(forHTTPHeaderField: "Content-Type")?
        .hasPrefix("application/x-www-form-urlencoded") == true)
    let form = formItems(OIDCStub.recordedBody("/token"))
    #expect(form["code"] == "auth-code")
    #expect(form["grant_type"] == "authorization_code")
    #expect(form["redirect_uri"] == query["redirect_uri"])
    #expect(form["client_id"] == "client")
    #expect(form["client_secret"] == "secret")
    #expect(form.count == 5)
  }

  @Test func authorizeLoginRequiredIsInvalidCredentials() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/authorize",
      .init(statusCode: 302, headers: ["Location": "typie:///authorize?error=login_required"]))
    let client = try makeClient()
    await #expect(throws: InvalidCredentialsError.self) {
      _ = try await client.exchange(sessionToken: "session")
    }
  }

  @Test func authorizeOtherErrorIsMalformedResponse() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/authorize",
      .init(statusCode: 302, headers: ["Location": "typie:///authorize?error=server_error"]))
    let client = try makeClient()
    await #expect(throws: HTTPError.malformedResponse("/authorize: server_error")) {
      _ = try await client.exchange(sessionToken: "session")
    }
  }

  @Test func authorizeNonRedirectIsStatus() async throws {
    OIDCStub.reset()
    OIDCStub.set("/authorize", .init(statusCode: 200))
    let client = try makeClient()
    await #expect(throws: HTTPError.status(200)) {
      _ = try await client.exchange(sessionToken: "session")
    }
    OIDCStub.set("/authorize", .init(statusCode: 500))
    await #expect(throws: HTTPError.status(500)) {
      _ = try await client.exchange(sessionToken: "session")
    }
  }

  @Test func authorizeRedirectWithoutCodeIsMalformedResponse() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/authorize", .init(statusCode: 302, headers: ["Location": "typie:///authorize?state=x"]))
    let client = try makeClient()
    await #expect(
      throws: HTTPError.malformedResponse("/authorize: no code in redirect response")
    ) {
      _ = try await client.exchange(sessionToken: "session")
    }
  }

  @Test func authorizeRedirectWithoutLocationIsMalformedResponse() async throws {
    OIDCStub.reset()
    OIDCStub.set("/authorize", .init(statusCode: 302))
    let client = try makeClient()
    await #expect(
      throws: HTTPError.malformedResponse("/authorize: no Location header in redirect response")
    ) {
      _ = try await client.exchange(sessionToken: "session")
    }
  }

  @Test func cancellationPropagatesAsCancellationError() async throws {
    OIDCStub.reset()
    OIDCStub.set("/authorize", .init(failure: .cancelled))
    let client = try makeClient()
    await #expect(throws: CancellationError.self) {
      _ = try await client.exchange(sessionToken: "session")
    }
  }

  @Test func tokenInvalidGrantIsInvalidCredentials() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/authorize",
      .init(statusCode: 302, headers: ["Location": "typie:///authorize?code=auth-code"]))
    OIDCStub.set(
      "/token",
      .init(
        statusCode: 400, headers: ["Content-Type": "application/json"],
        body: Data(#"{"error":"invalid_grant"}"#.utf8)))
    let client = try makeClient()
    await #expect(throws: InvalidCredentialsError.self) {
      _ = try await client.exchange(sessionToken: "session")
    }
  }

  @Test func tokenServerErrorIsStatus() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/authorize",
      .init(statusCode: 302, headers: ["Location": "typie:///authorize?code=auth-code"]))
    OIDCStub.set(
      "/token",
      .init(
        statusCode: 500, headers: ["Content-Type": "application/json"],
        body: Data(#"{"error":"invalid_grant"}"#.utf8)))
    let client = try makeClient()
    await #expect(throws: HTTPError.status(500)) {
      _ = try await client.exchange(sessionToken: "session")
    }
  }

  @Test func tokenWithoutAccessTokenIsMalformedResponse() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/authorize",
      .init(statusCode: 302, headers: ["Location": "typie:///authorize?code=auth-code"]))
    OIDCStub.set(
      "/token",
      .init(statusCode: 200, headers: ["Content-Type": "application/json"], body: Data("{}".utf8)))
    let client = try makeClient()
    let error = await capturedError { _ = try await client.exchange(sessionToken: "session") }
    guard case .malformedResponse = error as? HTTPError else {
      Issue.record("expected malformedResponse")
      return
    }
  }

  @Test func transportFailureIsNetworkError() async throws {
    OIDCStub.reset()
    OIDCStub.set("/authorize", .init(failure: .notConnectedToInternet))
    let client = try makeClient()
    let error = await capturedError { _ = try await client.exchange(sessionToken: "session") }
    guard case .network(let message) = error as? HTTPError else {
      Issue.record("expected network error")
      return
    }
    #expect(!message.isEmpty)
  }

  @Test func logoutSendsRedirectURIAndCookieAndIgnoresFailure() async throws {
    OIDCStub.reset()
    OIDCStub.set("/logout", .init(statusCode: 500, body: Data("boom".utf8)))
    let client = try makeClient()
    await client.logout(sessionToken: "session")
    let request = try #require(OIDCStub.recordedRequest("/logout"))
    #expect(request.httpMethod == "GET")
    #expect(request.url?.host == "auth.example.test")
    let query = queryItems(request)
    #expect(query["redirect_uri"] == "typie:///")
    #expect(query.count == 1)
    #expect(request.value(forHTTPHeaderField: "Cookie") == "typie-st=session")
  }

  @Test func fetchMeParsesUser() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/graphql",
      .init(
        statusCode: 200, headers: ["Content-Type": "application/json"],
        body: Data(#"{"data":{"me":{"id":"u1"}}}"#.utf8)))

    let me = try await makeClient().fetchMe(accessToken: "access")
    #expect(me == Me(id: "u1"))

    let request = try #require(OIDCStub.recordedRequest("/graphql"))
    #expect(request.httpMethod == "POST")
    #expect(request.url?.host == "api.example.test")
    #expect(request.value(forHTTPHeaderField: "Authorization") == "Bearer access")
    #expect(request.value(forHTTPHeaderField: "Content-Type") == "application/json")
    let names = (request.allHTTPHeaderFields ?? [:]).keys.map { $0.lowercased() }
    #expect(!names.contains { $0.hasPrefix("x-device") })
    let body = try #require(OIDCStub.recordedBody("/graphql"))
    let decoded = try JSONDecoder().decode([String: String].self, from: body)
    #expect(decoded == ["query": "query AuthService_Me { me { id } }"])
  }

  @Test func fetchMeWithoutUserUsesFirstErrorMessage() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/graphql",
      .init(
        statusCode: 200, headers: ["Content-Type": "application/json"],
        body: Data(#"{"data":{"me":null},"errors":[{"message":"forbidden"}]}"#.utf8)))
    let client = try makeClient()
    await #expect(throws: HTTPError.malformedResponse("/graphql me: forbidden")) {
      _ = try await client.fetchMe(accessToken: "access")
    }
  }

  @Test func fetchMeWithoutUserOrErrorsReportsNoData() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/graphql",
      .init(
        statusCode: 200, headers: ["Content-Type": "application/json"],
        body: Data(#"{"data":{"me":null}}"#.utf8)))
    let client = try makeClient()
    await #expect(throws: HTTPError.malformedResponse("/graphql me: no data")) {
      _ = try await client.fetchMe(accessToken: "access")
    }
  }

  @Test func fetchMeUnauthorizedIsStatus() async throws {
    OIDCStub.reset()
    OIDCStub.set(
      "/graphql",
      .init(
        statusCode: 401, headers: ["Content-Type": "application/json"],
        body: Data(#"{"errors":[{"message":"unauthorized"}]}"#.utf8)))
    let client = try makeClient()
    await #expect(throws: HTTPError.status(401)) {
      _ = try await client.fetchMe(accessToken: "access")
    }
  }
}
