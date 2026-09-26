import EditorFFI
import Foundation
import Testing

@testable import Editor

@MainActor @Suite final class FontLoaderTests {
  private let target = FakeFontTarget()
  private let network = FakeFontNetwork()
  private let clock = SleepGate()
  private let cacheDirectory = FileManager.default.temporaryDirectory.appending(
    path: "font-cache-\(UUID().uuidString)")
  private lazy var loader = FontLoader(
    target: target, fetch: network.fetch, sleep: clock.sleep, cacheDirectory: cacheDirectory)

  deinit {
    try? FileManager.default.removeItem(at: cacheDirectory)
  }

  private func bytes(_ url: URL) -> Data {
    Data(url.absoluteString.utf8)
  }

  @Test func applyHandsTheListToTheEngine() throws {
    try loader.apply([
      EditorFontFamily(
        name: "P", source: .default,
        fonts: [
          .init(weight: 400, url: fontURL("P", 400), hash: "h1"),
          .init(weight: 700, url: fontURL("P", 700), hash: "h2"),
        ]),
      EditorFontFamily(
        name: "U", source: .user, fonts: [.init(weight: 400, url: fontURL("U", 400), hash: "u1")]),
      EditorFontFamily(
        name: "F", source: .fallback,
        fonts: [.init(weight: 400, url: fontURL("F", 400), hash: "f1")]),
    ])
    #expect(
      target.lists == [
        [
          FontFamily(
            name: "P", source: .default,
            weights: [FontWeight(value: 400, hash: "h1"), FontWeight(value: 700, hash: "h2")]),
          FontFamily(name: "U", source: .user, weights: [FontWeight(value: 400, hash: "u1")]),
          FontFamily(name: "F", source: .fallback, weights: [FontWeight(value: 400, hash: "f1")]),
        ]
      ])
  }

  @Test func loadDispatchedBeforeAHashRollbackCommitsNothing() async throws {
    let manifest = partURL("P", 400, "h1", "manifest.v2")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.hold(manifest)
    loader.receive([missing("P", 400, required: [.manifest])])
    try await waitOnMain { self.network.isWaiting(manifest) }
    try loader.apply([fontFamily("P", 400, "h2")])
    try loader.apply([fontFamily("P", 400, "h1")])
    network.releaseAll()
    await loader.settle()
    #expect(target.commits.isEmpty)
    #expect(network.requests == [manifest])
  }

  @Test func hashChangeDropsLoadedPartsRetriesAndQueuedPrefetch() async throws {
    let fillers = [100, 101, 102, 103].map { partURL("Q", 400, "g1", "chunks/\($0)") }
    let failing = partURL("P", 400, "h1", "chunks/3")
    try loader.apply([fontFamily("P", 400, "h1"), fontFamily("Q", 400, "g1")])
    for url in fillers { network.hold(url) }
    await loader.handle(
      family: "Q", weight: 400, required: [],
      prefetch: [100, 101, 102, 103].map { FontData.chunk(id: $0) })
    try await waitOnMain { fillers.allSatisfy(self.network.isWaiting) }
    network.fail(failing)
    clock.hold(.seconds(2))
    await loader.handle(
      family: "P", weight: 400, required: [.manifest, .base, .chunk(id: 3)],
      prefetch: [.chunk(id: 5)])
    try await waitOnMain { self.clock.isWaiting(.seconds(2)) }
    try loader.apply([fontFamily("P", 400, "h2"), fontFamily("Q", 400, "g1")])
    await loader.handle(
      family: "P", weight: 400, required: [.manifest, .base, .chunk(id: 3)], prefetch: [])
    network.releaseAll()
    clock.releaseAll()
    await loader.settle()
    #expect(network.count(failing) == 3)
    #expect(network.count(partURL("P", 400, "h1", "chunks/5")) == 0)
    for path in ["manifest.v2", "base", "chunks/3"] {
      #expect(network.count(partURL("P", 400, "h2", path)) == 1)
    }
  }

  @Test func hashChangeKeepsOtherFontsLoaded() async throws {
    try loader.apply([fontFamily("P", 400, "h1"), fontFamily("O", 700, "h9")])
    await loader.handle(family: "O", weight: 700, required: [.base], prefetch: [])
    try loader.apply([fontFamily("P", 400, "h2"), fontFamily("O", 700, "h9")])
    await loader.handle(family: "O", weight: 700, required: [.base], prefetch: [])
    #expect(network.requests == [partURL("O", 700, "h9", "base")])
    #expect(target.commits.count == 1)
  }

  @Test func concurrentRequestsForOnePartShareOneFetch() async throws {
    let manifest = partURL("P", 400, "h1", "manifest.v2")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.hold(manifest)
    loader.receive([missing("P", 400, required: [.manifest])])
    loader.receive([missing("P", 400, required: [.manifest])])
    try await waitOnMain { self.network.isWaiting(manifest) }
    network.releaseAll()
    await loader.settle()
    #expect(network.requests == [manifest])
    #expect(target.commits.map(\.part) == [.manifest])
  }

  @Test func sharedFetchDiscardedAsStaleCommitsNothingForEitherRequest() async throws {
    let manifest = partURL("P", 400, "h1", "manifest.v2")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.hold(manifest)
    loader.receive([missing("P", 400, required: [.manifest])])
    loader.receive([missing("P", 400, required: [.manifest])])
    try await waitOnMain { self.network.isWaiting(manifest) }
    try loader.apply([fontFamily("P", 400, "h2")])
    network.releaseAll()
    await loader.settle()
    #expect(network.requests == [manifest])
    #expect(target.commits.isEmpty)
    #expect(clock.requests.isEmpty)
  }

  @Test func retryJoinsARunningLoadAndEndsWhenItSucceeds() async throws {
    let base = partURL("P", 400, "h1", "base")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.fail(base)
    clock.hold(.seconds(2))
    await loader.handle(family: "P", weight: 400, required: [.base], prefetch: [])
    try await waitOnMain { self.clock.isWaiting(.seconds(2)) }
    network.serve(base)
    network.hold(base)
    loader.receive([missing("P", 400, required: [.base])])
    try await waitOnMain { self.network.isWaiting(base) }
    clock.release(.seconds(2))
    try await Task.sleep(for: .milliseconds(50))
    #expect(network.count(base) == 4)
    network.releaseAll()
    await loader.settle()
    #expect(network.count(base) == 4)
    #expect(target.commits.map(\.part) == [.base])
    #expect(clock.requests == [.milliseconds(200), .milliseconds(400), .seconds(2)])
  }

  @Test func retryChainRunsFiveStepsAtGrowingIntervalsThenStops() async throws {
    let base = partURL("P", 400, "h1", "base")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.fail(base)
    loader.receive([missing("P", 400, required: [.base])])
    await loader.settle()
    let pauses: [Duration] = [.milliseconds(200), .milliseconds(400)]
    let steps: [Duration] = [2, 4, 8, 16, 30].map { .seconds($0) }
    #expect(clock.requests == pauses + steps.flatMap { [$0] + pauses })
    #expect(network.count(base) == 18)
    #expect(target.commits.isEmpty)
  }

  @Test func retryChainForAReplacedHashNeverFetchesAgain() async throws {
    let old = partURL("P", 400, "h1", "base")
    let new = partURL("P", 400, "h2", "base")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.fail(old)
    network.fail(new)
    clock.hold(.seconds(2))
    await loader.handle(family: "P", weight: 400, required: [.base], prefetch: [])
    try await waitOnMain { self.clock.isWaiting(.seconds(2)) }
    try loader.apply([fontFamily("P", 400, "h2")])
    await loader.handle(family: "P", weight: 400, required: [.base], prefetch: [])
    try await waitOnMain { self.clock.waitingCount(.seconds(2)) == 2 }
    clock.releaseAll()
    await loader.settle()
    #expect(network.count(old) == 3)
    #expect(network.count(new) == 18)
    #expect(
      clock.requests.filter { $0 >= .seconds(1) } == [2, 2, 4, 8, 16, 30].map { .seconds($0) })
  }

  @Test func prefetchRequestedAgainWhileRunningRunsOnce() async throws {
    let chunk = partURL("P", 400, "h1", "chunks/5")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.hold(chunk)
    await loader.handle(family: "P", weight: 400, required: [], prefetch: [.chunk(id: 5)])
    try await waitOnMain { self.network.isWaiting(chunk) }
    await loader.handle(family: "P", weight: 400, required: [], prefetch: [.chunk(id: 5)])
    network.releaseAll()
    await loader.settle()
    #expect(network.count(chunk) == 1)
    #expect(target.commits.map(\.part) == [.chunk(id: 5)])
  }

  @Test func hashChangeAfterACommitLoadsTheNewHashAndForgetsTheOld() async throws {
    let first = partURL("P", 400, "h1", "manifest.v2")
    let second = partURL("P", 400, "h2", "manifest.v2")
    try loader.apply([fontFamily("P", 400, "h1")])
    await loader.handle(family: "P", weight: 400, required: [.manifest], prefetch: [])
    try loader.apply([fontFamily("P", 400, "h2")])
    await loader.handle(family: "P", weight: 400, required: [.manifest], prefetch: [])
    try loader.apply([fontFamily("P", 400, "h1")])
    await loader.handle(family: "P", weight: 400, required: [.manifest], prefetch: [])
    #expect(network.requests == [first, second])
    #expect(target.commits.map(\.data) == [bytes(first), bytes(second), bytes(first)])
  }

  @Test func prefetchStartsInPriorityOrder() async throws {
    let fillers = [100, 101, 102, 103].map { partURL("Q", 400, "g1", "chunks/\($0)") }
    let ordered = ["manifest.v2", "base", "chunks/2", "chunks/9"].map {
      partURL("P", 400, "h1", $0)
    }
    try loader.apply([fontFamily("P", 400, "h1"), fontFamily("Q", 400, "g1")])
    for url in fillers + ordered { network.hold(url) }
    await loader.handle(
      family: "Q", weight: 400, required: [],
      prefetch: [100, 101, 102, 103].map { FontData.chunk(id: $0) })
    try await waitOnMain { fillers.allSatisfy(self.network.isWaiting) }
    await loader.handle(
      family: "P", weight: 400, required: [],
      prefetch: [.chunk(id: 9), .chunk(id: 2), .manifest, .base])
    for (filler, next) in zip(fillers, ordered) {
      network.release(filler)
      try await waitOnMain { self.network.isWaiting(next) }
    }
    network.releaseAll()
    await loader.settle()
    #expect(network.requests.filter { ordered.contains($0) } == ordered)
  }

  @Test func requiredPartTriesThreeTimesWithGrowingPauses() async throws {
    let base = partURL("P", 400, "h1", "base")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.fail(base)
    clock.hold(.seconds(2))
    await loader.handle(family: "P", weight: 400, required: [.base], prefetch: [])
    #expect(network.count(base) == 3)
    try await waitOnMain { self.clock.isWaiting(.seconds(2)) }
    #expect(clock.requests == [.milliseconds(200), .milliseconds(400), .seconds(2)])
    clock.releaseAll()
    await loader.settle()
  }

  @Test func manifestFallsBackToV1WhenV2IsUnavailable() async throws {
    let v2 = partURL("P", 400, "h1", "manifest.v2")
    let v1 = partURL("P", 400, "h1", "manifest.v1")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.fail(v2)
    await loader.handle(family: "P", weight: 400, required: [.manifest], prefetch: [])
    #expect(network.requests == [v2, v1])
    #expect(
      target.commits == [.init(part: .manifest, family: "P", weight: 400, data: bytes(v1))])
    #expect(clock.requests.isEmpty)
  }

  @Test func manifestFallsBackToV1WhenTheEngineRejectsV2() async throws {
    let v2 = partURL("P", 400, "h1", "manifest.v2")
    let v1 = partURL("P", 400, "h1", "manifest.v1")
    try loader.apply([fontFamily("P", 400, "h1")])
    target.rejected = [bytes(v2)]
    await loader.handle(family: "P", weight: 400, required: [.manifest], prefetch: [])
    #expect(network.requests == [v2, v1])
    #expect(target.commits.map(\.data) == [bytes(v1)])
    #expect(clock.requests.isEmpty)
  }

  @Test func requiredPartsLoadManifestThenBaseThenChunks() async throws {
    let manifest = partURL("P", 400, "h1", "manifest.v2")
    let base = partURL("P", 400, "h1", "base")
    let chunks = [partURL("P", 400, "h1", "chunks/0"), partURL("P", 400, "h1", "chunks/1")]
    try loader.apply([fontFamily("P", 400, "h1")])
    network.hold(manifest)
    network.hold(base)
    loader.receive([
      missing("P", 400, required: [.chunk(id: 1), .base, .manifest, .chunk(id: 0)])
    ])
    try await waitOnMain { self.network.isWaiting(manifest) }
    #expect(network.requests == [manifest])
    network.release(manifest)
    try await waitOnMain { self.network.isWaiting(base) }
    #expect(network.requests == [manifest, base])
    network.release(base)
    await loader.settle()
    #expect(Set(network.requests.dropFirst(2)) == Set(chunks))
    #expect(target.commits.prefix(2).map(\.part) == [.manifest, .base])
    #expect(Set(target.commits.dropFirst(2).map(\.part)) == [.chunk(id: 0), .chunk(id: 1)])
  }

  @Test func failedStageDoesNotBlockTheNextStage() async throws {
    try loader.apply([fontFamily("P", 400, "h1")])
    network.fail(partURL("P", 400, "h1", "manifest.v2"))
    network.fail(partURL("P", 400, "h1", "manifest.v1"))
    clock.hold(.seconds(2))
    await loader.handle(
      family: "P", weight: 400, required: [.manifest, .base, .chunk(id: 0)], prefetch: [])
    #expect(target.commits.map(\.part) == [.base, .chunk(id: 0)])
    clock.releaseAll()
    await loader.settle()
  }

  @Test func dataTheEngineRejectsIsRemovedFromTheCache() async throws {
    let base = partURL("P", 400, "h1", "base")
    let cache = FontCache(directory: cacheDirectory)
    let rejected = Data([0])
    await cache.write(rejected, for: base)
    target.rejected = [rejected]
    try loader.apply([fontFamily("P", 400, "h1")])
    await loader.handle(family: "P", weight: 400, required: [.base], prefetch: [])
    #expect(network.requests == [base])
    #expect(target.commits.map(\.data) == [bytes(base)])
    #expect(await cache.read(base) == bytes(base))
    #expect(clock.requests == [.milliseconds(200)])
  }

  @Test func cachedDataIsUsedWithoutFetching() async throws {
    let manifest = partURL("P", 400, "h1", "manifest.v2")
    let cached = Data([1])
    await FontCache(directory: cacheDirectory).write(cached, for: manifest)
    try loader.apply([fontFamily("P", 400, "h1")])
    await loader.handle(family: "P", weight: 400, required: [.manifest], prefetch: [])
    #expect(network.requests.isEmpty)
    #expect(target.commits.map(\.data) == [cached])
  }

  @Test func cacheFilesAreNamedBySHA256OfTheURL() {
    let file = FontCache(directory: cacheDirectory).file(for: URL(string: "https://fonts.test/a")!)
    #expect(
      file
        == cacheDirectory.appending(
          path: "a8322c822f3ee90351cf50bc0f17495af7d2c69831ce15b5d2214394cb51de74"))
  }

  @Test func prefetchTriesOnce() async throws {
    let chunk = partURL("P", 400, "h1", "chunks/3")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.fail(chunk)
    await loader.handle(family: "P", weight: 400, required: [], prefetch: [.chunk(id: 3)])
    await loader.settle()
    #expect(network.count(chunk) == 1)
    #expect(clock.requests.isEmpty)
    #expect(target.commits.isEmpty)
  }

  @Test func requestsForFontsMissingFromTheListAreIgnored() async throws {
    try loader.apply([fontFamily("P", 400, "h1")])
    await loader.handle(
      family: "Z", weight: 400, required: [.manifest, .base], prefetch: [.chunk(id: 0)])
    await loader.handle(family: "P", weight: 700, required: [.manifest], prefetch: [])
    await loader.settle()
    #expect(network.requests.isEmpty)
    #expect(target.commits.isEmpty)
  }

  @Test func requiredRequestForARunningPrefetchJoinsIt() async throws {
    let chunk = partURL("P", 400, "h1", "chunks/5")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.hold(chunk)
    await loader.handle(family: "P", weight: 400, required: [], prefetch: [.chunk(id: 5)])
    try await waitOnMain { self.network.isWaiting(chunk) }
    loader.receive([missing("P", 400, required: [.chunk(id: 5)])])
    try await Task.sleep(for: .milliseconds(50))
    #expect(network.count(chunk) == 1)
    network.releaseAll()
    await loader.settle()
    #expect(network.count(chunk) == 1)
    #expect(target.commits.map(\.part) == [.chunk(id: 5)])
    #expect(clock.requests.isEmpty)
  }

  @Test func requestsAfterAHashRollbackShareTheNewLoad() async throws {
    let manifest = partURL("P", 400, "h1", "manifest.v2")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.hold(manifest)
    let stale = Task {
      await self.loader.handle(family: "P", weight: 400, required: [.manifest], prefetch: [])
    }
    try await waitOnMain { self.network.waitingCount(manifest) == 1 }
    try loader.apply([fontFamily("P", 400, "h2")])
    try loader.apply([fontFamily("P", 400, "h1")])
    loader.receive([missing("P", 400, required: [.manifest])])
    try await waitOnMain { self.network.waitingCount(manifest) == 2 }
    network.release(manifest)
    await stale.value
    loader.receive([missing("P", 400, required: [.manifest])])
    network.releaseAll()
    await loader.settle()
    #expect(network.count(manifest) == 2)
    #expect(target.commits.map(\.part) == [.manifest])
  }

  @Test func retryChainFromBeforeAHashRollbackNeverFetchesAgain() async throws {
    let base = partURL("P", 400, "h1", "base")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.fail(base)
    clock.hold(.seconds(2))
    await loader.handle(family: "P", weight: 400, required: [.base], prefetch: [])
    try await waitOnMain { self.clock.isWaiting(.seconds(2)) }
    try loader.apply([fontFamily("P", 400, "h2")])
    try loader.apply([fontFamily("P", 400, "h1")])
    await loader.handle(family: "P", weight: 400, required: [.base], prefetch: [])
    try await waitOnMain { self.clock.waitingCount(.seconds(2)) == 2 }
    clock.releaseAll()
    await loader.settle()
    #expect(network.count(base) == 21)
    #expect(
      clock.requests.filter { $0 >= .seconds(1) } == [2, 2, 4, 8, 16, 30].map { .seconds($0) })
  }

  @Test func retryFromBeforeAHashRollbackLeavesTheNewRetryRunning() async throws {
    let base = partURL("P", 400, "h1", "base")
    try loader.apply([fontFamily("P", 400, "h1")])
    network.fail(base)
    clock.hold(.seconds(2))
    await loader.handle(family: "P", weight: 400, required: [.base], prefetch: [])
    try await waitOnMain { self.clock.isWaiting(.seconds(2)) }
    network.serve(base)
    network.hold(base)
    clock.release(.seconds(2))
    try await waitOnMain { self.network.isWaiting(base) }
    try loader.apply([fontFamily("P", 400, "h2")])
    try loader.apply([fontFamily("P", 400, "h1")])
    network.unhold(base)
    network.fail(base)
    await loader.handle(family: "P", weight: 400, required: [.base], prefetch: [])
    try await waitOnMain { self.clock.isWaiting(.seconds(2)) }
    network.serve(base)
    network.release(base)
    try await Task.sleep(for: .milliseconds(50))
    clock.releaseAll()
    await loader.settle()
    #expect(target.commits.map(\.part) == [.base])
  }
}
