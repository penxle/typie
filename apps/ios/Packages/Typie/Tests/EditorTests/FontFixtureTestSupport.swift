import EditorFFI
import Foundation
import Testing

@testable import Editor

struct FontFixture {
  let directory: URL
  let family: EditorFontFamily
  let chunkCount: Int

  init() throws {
    let fixtures = try #require(Bundle.module.url(forResource: "Fixtures", withExtension: nil))
    directory = fixtures.appending(path: "Fonts")
    let builds = try FileManager.default.contentsOfDirectory(
      atPath: directory.appending(path: "SUIT-400").path(percentEncoded: false))
    try #require(builds.count == 1)
    let hash = builds[0]
    chunkCount = try FileManager.default.contentsOfDirectory(
      atPath: directory.appending(path: "SUIT-400/\(hash)/chunks").path(percentEncoded: false)
    ).count
    family = EditorFontFamily(
      name: "SUIT", source: .default,
      fonts: [.init(weight: 400, url: fontURL("SUIT", 400), hash: hash)])
  }

  func url(_ path: String) -> URL {
    partURL("SUIT", 400, family.fonts[0].hash, path)
  }

  func read(_ url: URL) throws -> Data {
    try Data(contentsOf: directory.appending(path: String(url.path().dropFirst())))
  }
}

func fontRequests(_ events: [EditorEvent], family: String) -> [EditorEvent] {
  events.filter {
    guard case .fontDataMissing(let name, _, _, _) = $0 else { return false }
    return name == family
  }
}

func fontDocument(_ family: String, _ children: [PlainNodeEntry]) -> PlainDoc {
  PlainDoc(
    root: PlainNodeEntry(
      node: .root(PlainRootNode(layoutMode: paginatedLayout)),
      modifiers: [.fontFamily: .fontFamily(value: family)], children: children))
}

@MainActor
func loadFontsUntilQuiet(
  engine: EditorEngine, loader: FontLoader, events: [EditorEvent], family: String
) async throws -> [EditorEvent] {
  var requests = fontRequests(events, family: family)
  var rounds = 0
  while !requests.isEmpty, rounds < 16 {
    loader.receive(requests)
    await loader.settle()
    requests = fontRequests(try engine.tick()?.events ?? [], family: family)
    rounds += 1
  }
  return requests
}
