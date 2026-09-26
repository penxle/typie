import EditorFFI
import Foundation

@testable import Editor

struct WaitTimeout: Error {}

@MainActor
func waitOnMain(_ condition: @MainActor () -> Bool) async throws {
  for _ in 0..<1000 {
    if condition() { return }
    try await Task.sleep(for: .milliseconds(2))
  }
  throw WaitTimeout()
}

@MainActor
final class FakeFontTarget: FontCommitTarget {
  struct Commit: Equatable {
    let part: FontData
    let family: String
    let weight: UInt16
    let data: Data
  }

  private(set) var lists: [[FontFamily]] = []
  private(set) var commits: [Commit] = []
  var rejected: Set<Data> = []

  func setFonts(_ families: [FontFamily]) throws {
    lists.append(families)
  }

  func addFont(_ part: FontData, family: String, weight: UInt16, data: Data) throws {
    if rejected.contains(data) { throw FakeFailure() }
    commits.append(Commit(part: part, family: family, weight: weight, data: data))
  }
}

@MainActor
final class FakeFontNetwork {
  private(set) var requests: [URL] = []
  private var failing: Set<URL> = []
  private var holding: Set<URL> = []
  private var waiting: [(url: URL, continuation: CheckedContinuation<Bool, Never>)] = []
  private let body: @MainActor (URL) throws -> Data

  init(body: @escaping @MainActor (URL) throws -> Data = { Data($0.absoluteString.utf8) }) {
    self.body = body
  }

  var fetch: @Sendable (URL) async throws -> Data {
    { [self] url in try await respond(url) }
  }

  func fail(_ url: URL) { failing.insert(url) }
  func serve(_ url: URL) { failing.remove(url) }
  func hold(_ url: URL) { holding.insert(url) }
  func unhold(_ url: URL) { holding.remove(url) }
  func count(_ url: URL) -> Int { requests.filter { $0 == url }.count }
  func isWaiting(_ url: URL) -> Bool { waitingCount(url) > 0 }
  func waitingCount(_ url: URL) -> Int { waiting.filter { $0.url == url }.count }

  func release(_ url: URL, succeed: Bool = true) {
    guard let index = waiting.firstIndex(where: { $0.url == url }) else { return }
    waiting.remove(at: index).continuation.resume(returning: succeed)
  }

  func releaseAll() {
    holding = []
    let released = waiting
    waiting = []
    for entry in released { entry.continuation.resume(returning: true) }
  }

  private func respond(_ url: URL) async throws -> Data {
    requests.append(url)
    if holding.contains(url) {
      let succeed = await withCheckedContinuation { waiting.append((url, $0)) }
      guard succeed else { throw FakeFailure() }
    }
    if failing.contains(url) { throw FakeFailure() }
    return try body(url)
  }
}

@MainActor
final class SleepGate {
  private(set) var requests: [Duration] = []
  private let strict: Bool
  private var holding: Set<Duration> = []
  private var waiting: [(duration: Duration, continuation: CheckedContinuation<Void, Never>)] = []

  init(strict: Bool = false) {
    self.strict = strict
  }

  var sleep: @Sendable (Duration) async throws -> Void {
    { [self] duration in await wait(duration) }
  }

  func hold(_ duration: Duration) { holding.insert(duration) }
  func isWaiting(_ duration: Duration) -> Bool { waitingCount(duration) > 0 }
  func waitingCount(_ duration: Duration) -> Int {
    waiting.filter { $0.duration == duration }.count
  }

  func release(_ duration: Duration) {
    guard let index = waiting.firstIndex(where: { $0.duration == duration }) else { return }
    waiting.remove(at: index).continuation.resume()
  }

  func releaseAll() {
    holding = []
    let released = waiting
    waiting = []
    for entry in released { entry.continuation.resume() }
  }

  private func wait(_ duration: Duration) async {
    requests.append(duration)
    guard strict || holding.contains(duration) else { return }
    await withCheckedContinuation { waiting.append((duration, $0)) }
  }
}

func fontURL(_ name: String, _ weight: UInt16) -> URL {
  URL(string: "https://fonts.test/\(name)-\(weight)")!
}

func partURL(_ name: String, _ weight: UInt16, _ hash: String, _ path: String) -> URL {
  URL(string: "https://fonts.test/\(name)-\(weight)/\(hash)/\(path)")!
}

func fontFamily(_ name: String, _ weight: UInt16, _ hash: String) -> EditorFontFamily {
  EditorFontFamily(
    name: name, source: .default,
    fonts: [.init(weight: weight, url: fontURL(name, weight), hash: hash)]
  )
}

func missing(
  _ family: String, _ weight: UInt16, required: [FontData] = [], prefetch: [FontData] = []
) -> EditorEvent {
  .fontDataMissing(family: family, weight: weight, required: required, prefetch: prefetch)
}
