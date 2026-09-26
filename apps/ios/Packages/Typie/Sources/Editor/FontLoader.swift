internal import EditorFFI
import Foundation

@MainActor
protocol FontCommitTarget: AnyObject {
  func setFonts(_ families: [FontFamily]) throws
  func addFont(_ part: FontData, family: String, weight: UInt16, data: Data) throws
}

@MainActor
public final class FontLoader {
  private static let prefetchConcurrency = 4
  private static let requiredAttempts = 3
  private static let prefetchAttempts = 1
  private static let attemptPause = Duration.milliseconds(200)
  private static let retrySteps = 5
  private static let retryBase = Duration.seconds(2)
  private static let retryCap = Duration.seconds(30)

  private struct FontKey: Hashable {
    let family: String
    let weight: UInt16
  }

  private struct LoadKey: Hashable {
    let font: FontKey
    let hash: String
    let part: FontData
  }

  private struct Path {
    let url: URL
    let hash: String
  }

  private struct Dispatch {
    let font: FontKey
    let hash: String
    let generation: Int
    let base: URL

    func key(_ part: FontData) -> LoadKey {
      LoadKey(font: font, hash: hash, part: part)
    }
  }

  private struct Prefetch {
    let part: FontData
    let dispatch: Dispatch
    let priority: Int
  }

  private let target: any FontCommitTarget
  private let fetch: @Sendable (URL) async throws -> Data
  private let sleep: @Sendable (Duration) async throws -> Void
  private let cache: FontCache

  private var paths: [FontKey: Path] = [:]
  private var generations: [FontKey: Int] = [:]
  private var loaded: Set<LoadKey> = []
  private var flights: [LoadKey: Task<Bool, any Error>] = [:]
  private var chains: [LoadKey: Int] = [:]
  private var pending: [Prefetch] = []
  private var queued: Set<LoadKey> = []
  private var prefetching = 0
  private var work: [Int: Task<Void, Never>] = [:]
  private var nextWork = 0

  public convenience init(
    resources: EditorResources, fetch: @escaping @Sendable (URL) async throws -> Data,
    cacheDirectory: URL
  ) {
    self.init(
      target: resources, fetch: fetch, sleep: { try await Task.sleep(for: $0) },
      cacheDirectory: cacheDirectory)
  }

  init(
    target: any FontCommitTarget, fetch: @escaping @Sendable (URL) async throws -> Data,
    sleep: @escaping @Sendable (Duration) async throws -> Void, cacheDirectory: URL
  ) {
    self.target = target
    self.fetch = fetch
    self.sleep = sleep
    cache = FontCache(directory: cacheDirectory)
  }

  public func apply(_ families: [EditorFontFamily]) throws {
    var next: [FontKey: Path] = [:]
    for family in families {
      for font in family.fonts {
        next[FontKey(family: family.name, weight: font.weight)] = Path(
          url: font.url, hash: font.hash)
      }
    }
    let changed = Set(
      next.keys.filter { paths[$0]?.hash != next[$0]?.hash } + paths.keys.filter { next[$0] == nil }
    )
    try target.setFonts(families.map(Self.ffi))
    paths = next
    purge(changed)
  }

  func receive(_ events: [EditorEvent]) {
    for event in events {
      guard case .fontDataMissing(let family, let weight, let required, let prefetch) = event
      else { continue }
      spawn {
        await self.handle(family: family, weight: weight, required: required, prefetch: prefetch)
      }
    }
  }

  func handle(family: String, weight: UInt16, required: [FontData], prefetch: [FontData]) async {
    let font = FontKey(family: family, weight: weight)
    guard let path = paths[font] else { return }
    let dispatch = Dispatch(
      font: font, hash: path.hash, generation: generations[font, default: 0],
      base: path.url.appending(path: path.hash))
    if required.contains(.manifest) {
      await loadRequired(.manifest, dispatch)
    }
    if required.contains(.base) {
      await loadRequired(.base, dispatch)
    }
    await withTaskGroup(of: Void.self) { group in
      for case .chunk(let id) in required {
        group.addTask { await self.loadRequired(.chunk(id: id), dispatch) }
      }
    }
    for part in prefetch {
      enqueue(part, dispatch)
    }
  }

  func settle() async {
    while let task = work.values.first {
      await task.value
    }
  }

  private func spawn(_ body: @escaping @MainActor () async -> Void) {
    let id = nextWork
    nextWork += 1
    work[id] = Task {
      await body()
      self.work[id] = nil
    }
  }

  private func loadRequired(_ part: FontData, _ dispatch: Dispatch) async {
    do {
      _ = try await load(part, dispatch, attempts: Self.requiredAttempts)
    } catch {
      scheduleRetry(part, dispatch)
    }
  }

  private func load(_ part: FontData, _ dispatch: Dispatch, attempts: Int) async throws -> Bool {
    let key = dispatch.key(part)
    if loaded.contains(key) {
      return true
    }
    if let flight = flights[key] {
      _ = try await flight.value
      return loaded.contains(key)
    }
    let flight = Task { try await self.fetchAndCommit(part, dispatch, attempts: attempts) }
    flights[key] = flight
    defer {
      if flights[key] == flight {
        flights[key] = nil
      }
    }
    return try await flight.value
  }

  private func fetchAndCommit(_ part: FontData, _ dispatch: Dispatch, attempts: Int) async throws
    -> Bool
  {
    for attempt in 1..<attempts {
      if let committed = try? await commitFirstAvailable(part, dispatch) {
        return committed
      }
      try await sleep(Self.attemptPause * (1 << (attempt - 1)))
    }
    return try await commitFirstAvailable(part, dispatch)
  }

  private func commitFirstAvailable(_ part: FontData, _ dispatch: Dispatch) async throws -> Bool {
    switch part {
    case .manifest:
      if let committed = try? await commit(part, dispatch, from: "manifest.v2") {
        return committed
      }
      return try await commit(part, dispatch, from: "manifest.v1")
    case .base:
      return try await commit(part, dispatch, from: "base")
    case .chunk(let id):
      return try await commit(part, dispatch, from: "chunks/\(id)")
    }
  }

  private func commit(_ part: FontData, _ dispatch: Dispatch, from path: String) async throws
    -> Bool
  {
    let url = dispatch.base.appending(path: path)
    let data = try await read(url)
    guard generations[dispatch.font, default: 0] == dispatch.generation else { return false }
    do {
      try target.addFont(
        part, family: dispatch.font.family, weight: dispatch.font.weight, data: data)
    } catch {
      await cache.remove(url)
      throw error
    }
    loaded.insert(dispatch.key(part))
    return true
  }

  private func read(_ url: URL) async throws -> Data {
    if let data = await cache.read(url) {
      return data
    }
    let data = try await fetch(url)
    await cache.write(data, for: url)
    return data
  }

  private func scheduleRetry(_ part: FontData, _ dispatch: Dispatch) {
    let key = dispatch.key(part)
    guard chains[key] != dispatch.generation else { return }
    chains[key] = dispatch.generation
    spawn { await self.retry(part, dispatch) }
  }

  private func retry(_ part: FontData, _ dispatch: Dispatch) async {
    let key = dispatch.key(part)
    for step in 0..<Self.retrySteps {
      do {
        try await sleep(min(Self.retryBase * (1 << step), Self.retryCap))
      } catch {
        break
      }
      guard chains[key] == dispatch.generation else { return }
      if loaded.contains(key) {
        break
      }
      if let flight = flights[key] {
        _ = try? await flight.value
        if loaded.contains(key) {
          break
        }
        continue
      }
      do {
        _ = try await load(part, dispatch, attempts: Self.requiredAttempts)
        break
      } catch {
        continue
      }
    }
    if chains[key] == dispatch.generation {
      chains[key] = nil
    }
  }

  private func enqueue(_ part: FontData, _ dispatch: Dispatch) {
    let key = dispatch.key(part)
    guard !loaded.contains(key), !queued.contains(key) else { return }
    let item = Prefetch(part: part, dispatch: dispatch, priority: Self.priority(part))
    queued.insert(key)
    let index = pending.firstIndex(where: { $0.priority > item.priority }) ?? pending.endIndex
    pending.insert(item, at: index)
    drain()
  }

  private func drain() {
    while prefetching < Self.prefetchConcurrency, !pending.isEmpty {
      let item = pending.removeFirst()
      let key = item.dispatch.key(item.part)
      if loaded.contains(key) {
        queued.remove(key)
        continue
      }
      prefetching += 1
      spawn {
        _ = try? await self.load(item.part, item.dispatch, attempts: Self.prefetchAttempts)
        self.prefetching -= 1
        self.queued.remove(key)
        self.drain()
      }
    }
  }

  private func purge(_ fonts: Set<FontKey>) {
    guard !fonts.isEmpty else { return }
    for font in fonts {
      generations[font, default: 0] += 1
    }
    loaded = loaded.filter { !fonts.contains($0.font) }
    flights = flights.filter { !fonts.contains($0.key.font) }
    chains = chains.filter { !fonts.contains($0.key.font) }
    pending.removeAll { fonts.contains($0.dispatch.font) }
    queued = queued.filter { !fonts.contains($0.font) }
  }

  private static func priority(_ part: FontData) -> Int {
    switch part {
    case .manifest: -2
    case .base: -1
    case .chunk(let id): Int(id)
    }
  }

  private static func ffi(_ family: EditorFontFamily) -> FontFamily {
    let source: FontFamilySource =
      switch family.source {
      case .default: .default
      case .user: .user
      case .fallback: .fallback
      }
    return FontFamily(
      name: family.name, source: source,
      weights: family.fonts.map { FontWeight(value: $0.weight, hash: $0.hash) })
  }
}
