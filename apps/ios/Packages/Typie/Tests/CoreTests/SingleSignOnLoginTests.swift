import Foundation
import Testing

@testable import Core

final class SingleSignOnLoginStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var responseBody = ""
  nonisolated(unsafe) private static var recordedBody: Data?

  static func reset(body: String) {
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

@Suite(.serialized) struct SingleSignOnLoginTests {
  private func makeLogin() throws -> SingleSignOnLogin {
    let configuration = HTTPSession.configuration()
    configuration.protocolClasses = [SingleSignOnLoginStub.self]
    let client = GraphQLClient.make(
      config: try makeTestConfig(),
      deviceHeaders: { DeviceHeaders.make(deviceID: "abc", model: "iPhone", systemName: "iOS") },
      accessToken: { nil }, onSessionCookie: { _ in }, configuration: configuration)
    return SingleSignOnLogin(client: client)
  }

  private func errorBody(extensions: String) -> String {
    #"{"data":null,"errors":[{"message":"server message","extensions":\#(extensions)}]}"#
  }

  private func input(of body: Data) throws -> [String: Any] {
    let json = try #require(try JSONSerialization.jsonObject(with: body) as? [String: Any])
    let variables = try #require(json["variables"] as? [String: Any])
    return try #require(variables["input"] as? [String: Any])
  }

  @Test func sendsProviderAndParamsAsJSONObject() async throws {
    SingleSignOnLoginStub.reset(body: #"{"data":{"authorizeSingleSignOn":""}}"#)
    let login = try makeLogin()
    try await login(SingleSignOnCredential(provider: .kakao, params: ["access_token": "tok"]))
    let input = try input(of: try #require(SingleSignOnLoginStub.lastBody))
    #expect(input["provider"] as? String == "KAKAO")
    #expect(input["params"] as? [String: String] == ["access_token": "tok"])
    #expect(input["referralCode"] == nil)
  }

  @Test func mapsEveryProviderToItsWireName() async throws {
    let expected: [SingleSignOnProvider: String] = [
      .google: "GOOGLE", .kakao: "KAKAO", .naver: "NAVER", .apple: "APPLE",
    ]
    for (provider, name) in expected {
      SingleSignOnLoginStub.reset(body: #"{"data":{"authorizeSingleSignOn":""}}"#)
      let login = try makeLogin()
      try await login(SingleSignOnCredential(provider: provider, params: ["code": "c"]))
      let input = try input(of: try #require(SingleSignOnLoginStub.lastBody))
      #expect(input["provider"] as? String == name)
    }
  }

  @Test func rethrowsTypieErrors() async throws {
    SingleSignOnLoginStub.reset(
      body: errorBody(
        extensions: #"{"type":"TypieError","code":"rate_limited","message":"too many"}"#))
    let login = try makeLogin()
    await #expect(throws: TypieError(code: "rate_limited", message: "too many")) {
      try await login(SingleSignOnCredential(provider: .google, params: ["code": "c"]))
    }
  }

  @Test func preservesMessageForNonTypieErrors() async throws {
    SingleSignOnLoginStub.reset(body: errorBody(extensions: #"{"type":"Other"}"#))
    let login = try makeLogin()
    do {
      try await login(SingleSignOnCredential(provider: .apple, params: ["code": "c"]))
      Issue.record("expected an error")
    } catch is TypieError {
      Issue.record("mapped to TypieError")
    } catch {
      #expect(error.localizedDescription == "server message")
    }
  }

  @Test func failsWhenResponseHasNeitherDataNorErrors() async throws {
    SingleSignOnLoginStub.reset(body: #"{"data":null}"#)
    let login = try makeLogin()
    await #expect(throws: HTTPError.malformedResponse("authorizeSingleSignOn: no data")) {
      try await login(SingleSignOnCredential(provider: .naver, params: ["access_token": "t"]))
    }
  }
}
