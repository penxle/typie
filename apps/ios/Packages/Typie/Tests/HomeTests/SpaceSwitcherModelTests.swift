import Apollo
import Foundation
import Testing

@testable import Core
@testable import Home

final class SpaceSwitcherStub: StubURLProtocol, @unchecked Sendable {
  private static let lock = NSLock()
  nonisolated(unsafe) private static var responses: [(status: Int, body: String)] = []
  nonisolated(unsafe) private static var served = 0

  static func reset(_ queue: [(status: Int, body: String)]) {
    lock.withLock {
      responses = queue
      served = 0
    }
  }

  override func startLoading() {
    let response = Self.lock.withLock { () -> (status: Int, body: String) in
      let index = min(Self.served, Self.responses.count - 1)
      Self.served += 1
      return Self.responses[index]
    }
    respond(
      status: response.status, headers: ["Content-Type": "application/json"],
      body: Data(response.body.utf8))
  }
}

private func sitesBody(_ ids: [String]) -> String {
  let sites = ids.map {
    #"{"__typename":"Site","id":"\#($0)","name":"n-\#($0)","url":"https://\#($0).example.test","logo":{"__typename":"Image","id":"img-\#($0)","url":"https://img.example.test/\#($0).png","width":64,"height":64}}"#
  }.joined(separator: ",")
  return #"{"data":{"me":{"__typename":"User","id":"user-1","sites":[\#(sites)]}}}"#
}

@MainActor @Suite(.serialized) struct SpaceSwitcherModelTests {
  private struct Context {
    let model: SpaceSwitcherModel
    let activeSite: ActiveSiteStore
    let created: CallRecorder
    let createCalls: CallRecorder
  }

  private func makeContext(
    storedSiteId: String?, sites: [[String]],
    responses: [(status: Int, body: String)]? = nil,
    create: @escaping @Sendable (String) async throws -> String
  ) throws -> Context {
    SpaceSwitcherStub.reset(responses ?? sites.map { (200, sitesBody($0)) })
    let preferences = UserScopedDefaults(
      defaults: UserDefaults(suiteName: "home-\(UUID().uuidString)")!)
    preferences.switchUser("user-1")
    if let storedSiteId { preferences.siteId = storedSiteId }
    let activeSite = ActiveSiteStore(preferences: preferences)
    if let storedSiteId { Task { await activeSite.publish(storedSiteId) } }
    let client = GraphQLClient.make(
      config: try makeTestConfig(), deviceHeaders: { [:] }, accessToken: { nil },
      onSessionCookie: { _ in }, store: ApolloStore(),
      configuration: stubbedConfiguration(SpaceSwitcherStub.self))
    let created = CallRecorder()
    let createCalls = CallRecorder()
    let model = SpaceSwitcherModel(
      query: WatchQuery(client: client, query: SpaceSwitcher_Query()),
      activeSite: activeSite,
      createSite: { name in
        createCalls.record(name)
        return try await create(name)
      },
      onCreated: { created.record("created") })
    return Context(
      model: model, activeSite: activeSite, created: created, createCalls: createCalls
    )
  }

  @Test func reconcilesStoredSiteMissingFromList() async throws {
    let context = try makeContext(storedSiteId: "site-9", sites: [["site-1", "site-2"]]) { _ in "" }
    try await waitOnMain { context.model.spaces.count == 2 }
    #expect(context.activeSite.siteId == "site-1")
    #expect(context.model.current?.id == "site-1")
  }

  @Test func keepsStoredSiteWhenPresent() async throws {
    let context = try makeContext(storedSiteId: "site-2", sites: [["site-1", "site-2"]]) { _ in "" }
    try await waitOnMain { context.model.spaces.count == 2 }
    #expect(context.activeSite.siteId == "site-2")
  }

  @Test func selectSwitchesActiveSite() async throws {
    let context = try makeContext(storedSiteId: "site-1", sites: [["site-1", "site-2"]]) { _ in "" }
    try await waitOnMain { context.model.spaces.count == 2 }
    context.model.select("site-2")
    #expect(context.activeSite.siteId == "site-2")
    #expect(context.model.current?.id == "site-2")
  }

  @Test(.timeLimit(.minutes(1))) func createSelectsNewSiteAfterRefetch() async throws {
    let context = try makeContext(
      storedSiteId: "site-1", sites: [["site-1"], ["site-1", "site-2"]]
    ) { _ in "site-2" }
    try await waitOnMain { context.model.spaces.count == 1 }
    let ok = await context.model.createSpace(name: " space ")
    #expect(ok)
    #expect(context.model.isCreating == false)
    #expect(context.createCalls.calls == ["space"])
    #expect(context.activeSite.siteId == "site-2")
    #expect(context.created.calls == ["created"])
  }

  @Test(.timeLimit(.minutes(1))) func createResolvesFalseWhenRefetchOmitsNewSite() async throws {
    let context = try makeContext(
      storedSiteId: "site-1", sites: [["site-1"], ["site-1"]]
    ) { _ in "site-2" }
    try await waitOnMain { context.model.spaces.count == 1 }
    let ok = await context.model.createSpace(name: "x")
    #expect(ok == false)
    #expect(context.created.calls.isEmpty)
    #expect(context.model.isCreating == false)
    #expect(context.activeSite.siteId == "site-1")
  }

  @Test(.timeLimit(.minutes(1))) func createResolvesFalseWhenRefetchFails() async throws {
    let context = try makeContext(
      storedSiteId: nil, sites: [], responses: [(500, "{}"), (500, "{}")]
    ) { _ in "site-2" }
    try await waitOnMain { context.model.loadFailed }
    let ok = await context.model.createSpace(name: "x")
    #expect(ok == false)
    #expect(context.created.calls.isEmpty)
    #expect(context.model.isCreating == false)
  }

  @Test func blankNameFallsBackToDefault() async throws {
    let context = try makeContext(
      storedSiteId: "site-1", sites: [["site-1"], ["site-1", "site-2"]]
    ) { _ in "site-2" }
    try await waitOnMain { context.model.spaces.count == 1 }
    _ = await context.model.createSpace(name: "   ")
    #expect(context.createCalls.calls == ["새 스페이스"])
  }

  @Test func createFailureResolvesFalse() async throws {
    let context = try makeContext(storedSiteId: "site-1", sites: [["site-1"]]) { _ in
      throw TypieError(code: "subscription_required", message: nil)
    }
    try await waitOnMain { context.model.spaces.count == 1 }
    let ok = await context.model.createSpace(name: "x")
    #expect(ok == false)
    #expect(context.created.calls.isEmpty)
    #expect(context.model.isCreating == false)
  }

  @Test func loadFailureWithoutDataFlagsLoadFailedAndRetryRecovers() async throws {
    let context = try makeContext(
      storedSiteId: nil, sites: [], responses: [(500, "{}")]
    ) { _ in "" }
    try await waitOnMain { context.model.loadFailed }
    #expect(context.model.spaces.isEmpty)
    SpaceSwitcherStub.reset([(200, sitesBody(["site-1"]))])
    context.model.retry()
    try await waitOnMain { context.model.spaces.count == 1 }
    #expect(context.model.loadFailed == false)
  }

  @Test func loadFailedClearsWhenDataArrives() async throws {
    let context = try makeContext(
      storedSiteId: nil, sites: [], responses: [(500, "{}"), (200, sitesBody(["site-1"]))]
    ) { _ in "" }
    try await waitOnMain { context.model.loadFailed }
    context.model.refetch()
    try await waitOnMain { context.model.spaces.count == 1 }
    #expect(context.model.loadFailed == false)
  }

  @Test func loadFailureWithDataIsSilent() async throws {
    let context = try makeContext(storedSiteId: "site-1", sites: [["site-1"]]) { _ in "" }
    try await waitOnMain { context.model.spaces.count == 1 }
    SpaceSwitcherStub.reset([(500, "{}")])
    context.model.refetch()
    try await Task.sleep(for: .milliseconds(60))
    #expect(context.model.loadFailed == false)
    #expect(context.model.spaces.count == 1)
  }
}
