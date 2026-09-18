import Foundation
import Testing

@testable import Core

final class EmailLoginStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var responseBody = #"{"data":{"loginWithEmail":true}}"#
  nonisolated(unsafe) private static var recordedBody: Data?

  static func reset(body: String = #"{"data":{"loginWithEmail":true}}"#) {
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

@Suite(.serialized) struct EmailLoginTests {
  private func makeLogin() throws -> EmailLogin {
    let client = GraphQLClient.make(
      config: try makeTestConfig(),
      deviceHeaders: { DeviceHeaders.make(deviceID: "abc", model: "iPhone", systemName: "iOS") },
      accessToken: { nil }, onSessionCookie: { _ in },
      configuration: stubbedConfiguration(EmailLoginStub.self))
    return EmailLogin(client: client)
  }

  private func errorBody(extensions: String) -> String {
    #"{"data":null,"errors":[{"message":"server message","extensions":\#(extensions)}]}"#
  }

  @Test func sendsInputVerbatim() async throws {
    EmailLoginStub.reset()
    let login = try makeLogin()
    try await login(email: "  Dev@Example.COM ", password: " Pass Word ")
    let body = try #require(EmailLoginStub.lastBody)
    let json = try #require(try JSONSerialization.jsonObject(with: body) as? [String: Any])
    let variables = try #require(json["variables"] as? [String: Any])
    let input = try #require(variables["input"] as? [String: Any])
    #expect(input["email"] as? String == "  Dev@Example.COM ")
    #expect(input["password"] as? String == " Pass Word ")
  }

  @Test func mapsInvalidCredentials() async throws {
    EmailLoginStub.reset(
      body: errorBody(extensions: #"{"type":"TypieError","code":"invalid_credentials"}"#))
    let login = try makeLogin()
    await #expect(throws: EmailLoginError.invalidCredentials) {
      try await login(email: "a@b.test", password: "x")
    }
  }

  @Test func mapsPasswordNotSet() async throws {
    EmailLoginStub.reset(
      body: errorBody(extensions: #"{"type":"TypieError","code":"password_not_set"}"#))
    let login = try makeLogin()
    await #expect(throws: EmailLoginError.passwordNotSet) {
      try await login(email: "a@b.test", password: "x")
    }
  }

  @Test func rethrowsOtherTypieErrors() async throws {
    EmailLoginStub.reset(
      body: errorBody(
        extensions: #"{"type":"TypieError","code":"rate_limited","message":"too many"}"#))
    let login = try makeLogin()
    await #expect(throws: TypieError(code: "rate_limited", message: "too many")) {
      try await login(email: "a@b.test", password: "x")
    }
  }

  @Test func fallsBackToErrorMessageWhenExtensionsHaveNone() async throws {
    EmailLoginStub.reset(
      body: errorBody(extensions: #"{"type":"TypieError","code":"rate_limited"}"#))
    let login = try makeLogin()
    await #expect(throws: TypieError(code: "rate_limited", message: "server message")) {
      try await login(email: "a@b.test", password: "x")
    }
  }

  @Test func preservesMessageForNonTypieErrors() async throws {
    EmailLoginStub.reset(body: errorBody(extensions: #"{"type":"Other"}"#))
    let login = try makeLogin()
    do {
      try await login(email: "a@b.test", password: "x")
      Issue.record("expected an error")
    } catch is TypieError {
      Issue.record("mapped to TypieError")
    } catch is EmailLoginError {
      Issue.record("mapped to EmailLoginError")
    } catch {
      #expect(error.localizedDescription == "server message")
    }
  }

  @Test func demotesTypieErrorWithoutCode() async throws {
    EmailLoginStub.reset(body: errorBody(extensions: #"{"type":"TypieError"}"#))
    let login = try makeLogin()
    do {
      try await login(email: "a@b.test", password: "x")
      Issue.record("expected an error")
    } catch is TypieError {
      Issue.record("mapped to TypieError")
    } catch {
      #expect(error.localizedDescription == "server message")
    }
  }

  @Test func describesTypieErrorWithItsMessage() async throws {
    #expect(
      TypieError(code: "rate_limited", message: "server message").localizedDescription
        == "server message")
    #expect(TypieError(code: "rate_limited", message: nil).localizedDescription == "rate_limited")
  }

  @Test func failsWhenResponseHasNeitherDataNorErrors() async throws {
    EmailLoginStub.reset(body: #"{"data":null}"#)
    let login = try makeLogin()
    await #expect(throws: HTTPError.malformedResponse("loginWithEmail: no data")) {
      try await login(email: "a@b.test", password: "x")
    }
  }

  @Test func succeedsWithoutInspectingReturnValue() async throws {
    EmailLoginStub.reset(body: #"{"data":{"loginWithEmail":false}}"#)
    let login = try makeLogin()
    try await login(email: "a@b.test", password: "x")
  }
}
