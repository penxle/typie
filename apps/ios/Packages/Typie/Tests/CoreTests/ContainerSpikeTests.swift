import FactoryKit
import FactoryTesting
import Testing

@testable import Core

@MainActor
final class SpikeModel {
  let id: Int
  let onSuccess: (@MainActor () -> Void)?

  init(id: Int = Int.random(in: 0..<1_000_000)) {
    self.id = id
    onSuccess = nil
  }

  init(onSuccess: @escaping @MainActor () -> Void) {
    id = Int.random(in: 0..<1_000_000)
    self.onSuccess = onSuccess
  }
}

final class SpikeProbe {
  var fired = false
}

private actor SpikeRendezvous {
  private let expected = 2
  private var arrived: [SpikeModel] = []
  private var waiting: [Int: CheckedContinuation<[SpikeModel], any Error>] = [:]
  private var nextTicket = 0

  func exchange(_ model: SpikeModel) async throws -> [SpikeModel] {
    arrived.append(model)
    guard arrived.count < expected else {
      let all = arrived
      let pending = waiting
      waiting.removeAll()
      for continuation in pending.values {
        continuation.resume(returning: all)
      }
      return all
    }

    let ticket = nextTicket
    nextTicket += 1

    return try await withTaskCancellationHandler {
      try await withCheckedThrowingContinuation { continuation in
        if Task.isCancelled {
          continuation.resume(throwing: CancellationError())
        } else {
          waiting[ticket] = continuation
        }
      }
    } onCancel: {
      Task { await self.cancelWaiter(ticket) }
    }
  }

  private func cancelWaiter(_ ticket: Int) {
    guard let continuation = waiting.removeValue(forKey: ticket) else { return }
    continuation.resume(throwing: CancellationError())
  }
}

private let spikeRendezvous = SpikeRendezvous()

extension Container {
  @MainActor
  var spikeSession: Factory<SpikeModel> {
    self { SpikeModel() }.scope(.session)
  }

  @MainActor
  var spikeSingleton: Factory<SpikeModel> {
    self { SpikeModel() }.singleton
  }

  @MainActor
  var spikeParameterized: ParameterFactory<@MainActor () -> Void, SpikeModel> {
    self { SpikeModel(onSuccess: $0) }
  }

  @MainActor
  var spikePromised: Factory<SpikeModel> {
    self { fatalError() }.singleton
  }
}

@MainActor
@Suite(.container) struct ContainerSpikeTests {
  @Test func sessionScopeCachesUntilReset() {
    let a = Container.shared.spikeSession()
    let b = Container.shared.spikeSession()
    #expect(a === b)
    Container.shared.manager.reset(scope: .session)
    let c = Container.shared.spikeSession()
    #expect(a !== c)
  }

  @Test func sessionResetLeavesSingletons() {
    let a = Container.shared.spikeSingleton()
    Container.shared.manager.reset(scope: .session)
    #expect(Container.shared.spikeSingleton() === a)
  }

  @Test func registrationOverridesFactory() {
    let unregistered = Container.shared.spikeSession()
    let stub = SpikeModel()
    Container.shared.spikeSession.register { stub }
    let resolved = Container.shared.spikeSession()
    #expect(resolved === stub)
    #expect(resolved !== unregistered)
  }

  @Test func parameterFactoryDeliversMainActorClosure() {
    let probe = SpikeProbe()
    let model = Container.shared.spikeParameterized { probe.fired = true }
    #expect(model.onSuccess != nil)
    model.onSuccess?()
    #expect(probe.fired)
  }

  @Test func registrationSatisfiesPromisedFactory() {
    let stub = SpikeModel()
    Container.shared.spikePromised.register { stub }
    #expect(Container.shared.spikePromised() === stub)
  }

  @Test(.timeLimit(.minutes(1))) func registrationIsInvisibleToPeerSuite() async throws {
    let stub = SpikeModel(id: 1)
    Container.shared.spikeSession.register { stub }
    let peers = try await spikeRendezvous.exchange(stub).filter { $0 !== stub }
    #expect(peers.count == 1)
    let resolved = Container.shared.spikeSession()
    #expect(resolved === stub)
    #expect(peers.allSatisfy { $0.id != resolved.id })
  }
}

@MainActor
@Suite(.container) struct ContainerSpikeIsolationTests {
  @Test(.timeLimit(.minutes(1))) func registrationIsInvisibleToPeerSuite() async throws {
    let stub = SpikeModel(id: 2)
    Container.shared.spikeSession.register { stub }
    let peers = try await spikeRendezvous.exchange(stub).filter { $0 !== stub }
    #expect(peers.count == 1)
    let resolved = Container.shared.spikeSession()
    #expect(resolved === stub)
    #expect(peers.allSatisfy { $0.id != resolved.id })
  }
}
