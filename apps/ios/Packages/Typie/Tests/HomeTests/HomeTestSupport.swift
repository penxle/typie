import Foundation

@testable import Core

struct WaitTimeout: Error {}

@MainActor
func waitOnMain(_ condition: @MainActor () -> Bool) async throws {
  for _ in 0..<1000 {
    if condition() { return }
    try await Task.sleep(for: .milliseconds(2))
  }
  throw WaitTimeout()
}

func makeTestConfig() throws -> AppConfig {
  try AppConfig(
    infoDictionary: [
      "API_URL": "https://api.example.test", "AUTH_URL": "https://auth.example.test",
      "OIDC_CLIENT_ID": "client", "KAKAO_NATIVE_APP_KEY": "kakao-key",
      "NAVER_CLIENT_ID": "naver-client",
    ], oidcClientSecret: "secret", naverClientSecret: "naver-secret")
}

func stubbedConfiguration(_ stub: URLProtocol.Type) -> URLSessionConfiguration {
  let configuration = HTTPSession.configuration()
  configuration.protocolClasses = [stub]
  return configuration
}

class StubURLProtocol: URLProtocol, @unchecked Sendable {
  override class func canInit(with request: URLRequest) -> Bool { true }
  override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
  override func stopLoading() {}

  func respond(status: Int = 200, headers: [String: String] = [:], body: Data? = nil) {
    let url = request.url!
    let response = HTTPURLResponse(
      url: url, statusCode: status, httpVersion: "HTTP/1.1", headerFields: headers)!
    client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
    if let body { client?.urlProtocol(self, didLoad: body) }
    client?.urlProtocolDidFinishLoading(self)
  }
}

final class CallRecorder: @unchecked Sendable {
  private let lock = NSLock()
  private var entries: [String] = []

  func record(_ entry: String) {
    lock.withLock { entries.append(entry) }
  }

  var calls: [String] {
    lock.withLock { entries }
  }
}
