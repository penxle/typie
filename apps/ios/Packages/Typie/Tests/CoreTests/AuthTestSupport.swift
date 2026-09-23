import Foundation

@testable import Core

struct WaitTimeout: Error {}

func waitUntil(_ condition: @Sendable () -> Bool) async throws {
  for _ in 0..<1000 {
    if condition() { return }
    try await Task.sleep(for: .milliseconds(2))
  }
  throw WaitTimeout()
}

@MainActor
func waitOnMain(_ condition: @MainActor () -> Bool) async throws {
  for _ in 0..<1000 {
    if condition() { return }
    try await Task.sleep(for: .milliseconds(2))
  }
  throw WaitTimeout()
}

func makeTestConfig() -> AppConfig {
  AppConfig(
    infoDictionary: [
      "API_URL": "https://api.example.test", "AUTH_URL": "https://auth.example.test",
      "WS_URL": "wss://api.example.test",
      "OIDC_CLIENT_ID": "client", "OIDC_CLIENT_SECRET": "secret",
      "KAKAO_NATIVE_APP_KEY": "kakao-key", "NAVER_CLIENT_ID": "naver-client",
      "NAVER_CLIENT_SECRET": "naver-secret",
    ])
}

func stubbedConfiguration(_ stub: URLProtocol.Type) -> URLSessionConfiguration {
  let configuration = HTTPSession.configuration()
  configuration.protocolClasses = [stub]
  return configuration
}

func withFreshDefaults(_ body: (UserDefaults) throws -> Void) rethrows {
  let name = "test-\(UUID().uuidString)"
  let defaults = UserDefaults(suiteName: name)!
  defer { defaults.removePersistentDomain(forName: name) }
  try body(defaults)
}

class StubURLProtocol: URLProtocol, @unchecked Sendable {
  override class func canInit(with request: URLRequest) -> Bool { true }
  override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }
  override func stopLoading() {}

  func respond(status: Int = 200, headers: [String: String] = [:], body: Data? = nil) {
    let url = request.url!
    let response = HTTPURLResponse(
      url: url, statusCode: status, httpVersion: "HTTP/1.1", headerFields: headers)!
    if status == 302, let target = headers["Location"].flatMap(URL.init(string:)) {
      client?.urlProtocol(
        self, wasRedirectedTo: URLRequest(url: target), redirectResponse: response)
    }
    client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
    if let body { client?.urlProtocol(self, didLoad: body) }
    client?.urlProtocolDidFinishLoading(self)
  }

  static func body(of request: URLRequest) -> Data? {
    if let body = request.httpBody { return body }
    guard let stream = request.httpBodyStream else { return nil }
    stream.open()
    defer { stream.close() }
    var data = Data()
    var buffer = [UInt8](repeating: 0, count: 4096)
    while stream.hasBytesAvailable {
      let read = stream.read(&buffer, maxLength: buffer.count)
      if read <= 0 { break }
      data.append(contentsOf: buffer[0..<read])
    }
    return data
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

actor TestGate {
  private var isOpen = false
  private var waiters: [CheckedContinuation<Void, Never>] = []

  func wait() async {
    if isOpen { return }
    await withCheckedContinuation { continuation in
      waiters.append(continuation)
    }
  }

  func open() {
    isOpen = true
    let pending = waiters
    waiters = []
    for continuation in pending { continuation.resume() }
  }
}

final class ConcurrencyProbe: @unchecked Sendable {
  private let lock = NSLock()
  private var current = 0
  private var peak = 0
  private var completed = 0

  func enter() {
    lock.withLock {
      current += 1
      peak = max(peak, current)
    }
  }

  func exit() {
    lock.withLock {
      current -= 1
      completed += 1
    }
  }

  var maxConcurrent: Int { lock.withLock { peak } }
  var finished: Int { lock.withLock { completed } }
}
