import Core
import FactoryKit
import FactoryTesting
import Foundation
import GraphQL
import Testing

@testable import Features

@MainActor
@Suite(.container) struct SitesStoreTests {
  private struct Context {
    let model: SitesStore
    let client: FakeGraphQLClient
    let activeSite: ActiveSiteStore
    let created: CallRecorder
    let preferences: TestPreferences
  }

  private func makeContext(storedSiteId: String?) -> Context {
    let client = FakeGraphQLClient()
    registerFake(client)
    let preferences = makeTestPreferences(siteId: storedSiteId)
    Container.shared.userPreferences.register { preferences.userPreferences }
    let created = CallRecorder()
    let model = Container.shared.sites()
    model.onCreated = { created.record("created") }
    return Context(
      model: model, client: client, activeSite: Container.shared.activeSite(), created: created,
      preferences: preferences)
  }

  private func loaded(_ storedSiteId: String?, _ ids: [String]) async throws -> Context {
    let context = makeContext(storedSiteId: storedSiteId)
    context.client.push(.success(await siteSwitcherData(ids)), for: SiteSwitcher_Query.self)
    try await waitOnMain { context.model.sites.count == ids.count }
    return context
  }

  private func createdInput(_ context: Context) throws -> CreateSiteInput {
    try #require(
      context.client.performed(SiteSwitcher_CreateSite_Mutation.self).last?.input)
  }

  @Test func reconcilesStoredSiteMissingFromList() async throws {
    let context = try await loaded("site-9", ["site-1", "site-2"])
    #expect(context.activeSite.siteId == "site-1")
    #expect(context.model.current?.id == "site-1")
  }

  @Test func keepsStoredSiteWhenPresent() async throws {
    let context = try await loaded("site-2", ["site-1", "site-2"])
    #expect(context.activeSite.siteId == "site-2")
  }

  @Test func mapsSiteFields() async throws {
    let context = try await loaded("site-1", ["site-1"])
    let site = try #require(context.model.sites.first)
    #expect(site.id == "site-1")
    #expect(site.name == "n-site-1")
    #expect(site.url == "https://site-1.example.test")
    #expect(Img.url(of: site.logo)?.absoluteString == "https://img.example.test/site-1.png")
  }

  @Test func selectSwitchesActiveSite() async throws {
    let context = try await loaded("site-1", ["site-1", "site-2"])
    context.model.select("site-2")
    #expect(context.activeSite.siteId == "site-2")
    #expect(context.model.current?.id == "site-2")
  }

  @Test func refetchAsksTheWatcherAgain() async throws {
    let context = try await loaded("site-1", ["site-1"])
    context.model.refetch()
    try await waitOnMain { context.client.refetchCount(of: SiteSwitcher_Query.self) == 1 }
  }

  @Test(.timeLimit(.minutes(1))) func createSelectsNewSiteAfterRefetch() async throws {
    let context = try await loaded("site-1", ["site-1"])
    context.client.stub(
      .success(await createdSiteData("site-2")), for: SiteSwitcher_CreateSite_Mutation.self)
    context.client.queue(
      .success(await siteSwitcherData(["site-1", "site-2"])), for: SiteSwitcher_Query.self)

    let ok = await context.model.create(name: " site ")

    #expect(ok)
    #expect(context.model.isCreating == false)
    #expect(context.activeSite.siteId == "site-2")
    #expect(context.created.calls == ["created"])
    #expect(try createdInput(context).name == "site")
  }

  @Test(.timeLimit(.minutes(1))) func createResolvesFalseWhenRefetchOmitsNewSite() async throws {
    let context = try await loaded("site-1", ["site-1"])
    context.client.stub(
      .success(await createdSiteData("site-2")), for: SiteSwitcher_CreateSite_Mutation.self)
    context.client.queue(.success(await siteSwitcherData(["site-1"])), for: SiteSwitcher_Query.self)

    let ok = await context.model.create(name: "x")

    #expect(ok == false)
    #expect(context.created.calls.isEmpty)
    #expect(context.model.isCreating == false)
    #expect(context.activeSite.siteId == "site-1")
  }

  @Test(.timeLimit(.minutes(1))) func createResolvesFalseWhenRefetchFails() async throws {
    let context = makeContext(storedSiteId: nil)
    context.client.push(
      .failure(HTTPError.status(500)), for: SiteSwitcher_Query.self)
    try await waitOnMain { context.model.loadFailed }
    context.client.stub(
      .success(await createdSiteData("site-2")), for: SiteSwitcher_CreateSite_Mutation.self)
    context.client.queue(.failure(HTTPError.status(500)), for: SiteSwitcher_Query.self)

    let ok = await context.model.create(name: "x")

    #expect(ok == false)
    #expect(context.created.calls.isEmpty)
    #expect(context.model.isCreating == false)
  }

  @Test(.timeLimit(.minutes(1))) func blankNameFallsBackToDefault() async throws {
    let context = try await loaded("site-1", ["site-1"])
    context.client.stub(
      .success(await createdSiteData("site-2")), for: SiteSwitcher_CreateSite_Mutation.self)
    context.client.queue(
      .success(await siteSwitcherData(["site-1", "site-2"])), for: SiteSwitcher_Query.self)

    _ = await context.model.create(name: "   ")

    #expect(try createdInput(context).name == "새 스페이스")
  }

  @Test func createFailureResolvesFalse() async throws {
    let context = try await loaded("site-1", ["site-1"])
    context.client.stub(
      .failure(APIError(code: "subscription_required", message: nil)),
      for: SiteSwitcher_CreateSite_Mutation.self)

    let ok = await context.model.create(name: "x")

    #expect(ok == false)
    #expect(context.created.calls.isEmpty)
    #expect(context.model.isCreating == false)
    #expect(context.client.refetchCount(of: SiteSwitcher_Query.self) == 0)
  }

  @Test(.timeLimit(.minutes(1))) func secondCreateIsIgnoredWhileFirstIsInFlight() async throws {
    let context = try await loaded("site-1", ["site-1"])
    context.client.stub(
      .success(await createdSiteData("site-2")), for: SiteSwitcher_CreateSite_Mutation.self)
    context.client.queue(
      .success(await siteSwitcherData(["site-1", "site-2"])), for: SiteSwitcher_Query.self)
    let gate = context.client.hold()

    let first = Task { await context.model.create(name: "x") }
    try await waitOnMain { context.model.isCreating }

    #expect(await context.model.create(name: "y") == false)

    await gate.open()

    #expect(await first.value)
    #expect(context.client.performed(SiteSwitcher_CreateSite_Mutation.self).count == 1)
  }

  @Test func loadFailureWithoutDataFlagsLoadFailedAndDataRecovers() async throws {
    let context = makeContext(storedSiteId: nil)
    context.client.push(.failure(HTTPError.status(500)), for: SiteSwitcher_Query.self)
    try await waitOnMain { context.model.loadFailed }
    #expect(context.model.sites.isEmpty)

    context.client.push(.success(await siteSwitcherData(["site-1"])), for: SiteSwitcher_Query.self)
    try await waitOnMain { context.model.sites.count == 1 }

    #expect(context.model.loadFailed == false)
  }

  @Test(.timeLimit(.minutes(1))) func loadFailureWithDataIsSilent() async throws {
    let context = makeContext(storedSiteId: "site-1")
    let flags = CallRecorder()
    flags.record("\(context.model.loadFailed)")
    keepObserving(while: context.model) { [weak model = context.model] in
      guard let model else { return }
      flags.record("\(model.loadFailed)")
    }

    context.client.push(
      .success(await siteSwitcherData(["site-1", "site-2"])), for: SiteSwitcher_Query.self)
    try await waitOnMain { context.model.sites.count == 2 }

    context.client.push(.failure(HTTPError.status(500)), for: SiteSwitcher_Query.self)
    await drainMainActor()

    context.client.push(
      .success(await siteSwitcherData(["site-1", "site-2", "site-3"])),
      for: SiteSwitcher_Query.self)
    try await waitOnMain { context.model.sites.count == 3 }

    #expect(!flags.calls.contains("true"))
    #expect(context.model.sites.count == 3)
  }
}
