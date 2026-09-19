import Core
import FactoryKit
import FactoryTesting
import GraphQL
import Testing

@testable import Design
@testable import Features

@MainActor
@Suite(.container) struct CreateSiteModelTests {
  private struct Context {
    let model: CreateSiteModel
    let client: FakeGraphQLClient
    let preferences: TestPreferences
  }

  private func makeContext(create: Result<SiteSwitcher_CreateSite_Mutation.Data, any Error>)
    async throws -> Context
  {
    let client = FakeGraphQLClient()
    registerFake(client)
    let preferences = makeTestPreferences(siteId: "site-1")
    Container.shared.userPreferences.register { preferences.userPreferences }
    let sites = Container.shared.sites()
    client.push(.success(await siteSwitcherData(["site-1"])), for: SiteSwitcher_Query.self)
    try await waitOnMain { sites.sites.count == 1 }
    client.stub(create, for: SiteSwitcher_CreateSite_Mutation.self)
    client.queue(
      .success(await siteSwitcherData(["site-1", "site-2"])), for: SiteSwitcher_Query.self)
    return Context(model: CreateSiteModel(), client: client, preferences: preferences)
  }

  private func succeedingContext() async throws -> Context {
    try await makeContext(create: .success(await createdSiteData("site-2")))
  }

  @Test(.timeLimit(.minutes(1))) func submitPassesNameAndReportsSuccess() async throws {
    let context = try await succeedingContext()
    context.model.name.value = "site"

    #expect(await context.model.submit() == true)

    let performed = context.client.performed(SiteSwitcher_CreateSite_Mutation.self)
    #expect(performed.count == 1)
    #expect(performed.first?.input.name == "site")
    #expect(context.model.isSubmitting == false)
  }

  @Test func submitReportsCreateFailure() async throws {
    let context = try await makeContext(
      create: .failure(APIError(code: "subscription_required", message: nil)))

    #expect(await context.model.submit() == false)
    #expect(context.model.isSubmitting == false)
  }

  @Test(.timeLimit(.minutes(1))) func submitHoldsSubmittingUntilCreateEnds() async throws {
    let context = try await succeedingContext()
    let gate = context.client.hold()
    context.model.name.value = "site"

    let task = Task { await context.model.submit() }
    try await waitOnMain { context.client.performedMutations.count == 1 }

    #expect(context.model.isSubmitting)

    await gate.open()

    #expect(await task.value == true)
    #expect(context.model.isSubmitting == false)
  }

  @Test(.timeLimit(.minutes(1))) func secondSubmitIsIgnoredWhileInFlight() async throws {
    let context = try await succeedingContext()
    let gate = context.client.hold()
    context.model.name.value = "site"

    let task = Task { await context.model.submit() }
    try await waitOnMain { context.client.performedMutations.count == 1 }

    #expect(await context.model.submit() == nil)
    #expect(context.client.performed(SiteSwitcher_CreateSite_Mutation.self).count == 1)

    await gate.open()
    #expect(await task.value == true)
  }

  @Test func focusNameRequestsFieldFocus() async throws {
    let context = try await succeedingContext()
    #expect(context.model.name.focusRequest == 0)
    context.model.focusName()
    #expect(context.model.name.focusRequest == 1)
  }

  @Test(.timeLimit(.minutes(1))) func submitEndsEditingBeforeCreate() async throws {
    let context = try await succeedingContext()
    let gate = context.client.hold()
    context.model.name.isFocused = true

    let task = Task { await context.model.submit() }
    try await waitOnMain { context.client.performedMutations.count == 1 }

    #expect(context.model.name.resignRequest == 1)

    await gate.open()
    #expect(await task.value == true)
  }
}
