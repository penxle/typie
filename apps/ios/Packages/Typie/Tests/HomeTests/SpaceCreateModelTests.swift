import Testing

@testable import Design
@testable import Home

@MainActor private final class Gate {
  private var continuation: CheckedContinuation<Void, Never>?
  private var opened = false

  func wait() async {
    if opened { return }
    await withCheckedContinuation { self.continuation = $0 }
  }

  func open() {
    opened = true
    continuation?.resume()
    continuation = nil
  }
}

@MainActor private final class Holder {
  var model: SpaceCreateModel?
  var submittingDuringCreate: Bool?
  var resignRequestDuringCreate: Int?
}

@MainActor @Suite struct SpaceCreateModelTests {
  @Test func submitPassesNameAndHoldsSubmitting() async throws {
    let holder = Holder()
    let gate = Gate()
    let calls = CallRecorder()
    let model = SpaceCreateModel(create: { name in
      calls.record(name)
      holder.submittingDuringCreate = holder.model?.isSubmitting
      await gate.wait()
      return true
    })
    holder.model = model
    model.name.value = "space"
    let task = Task { await model.submit() }
    try await waitOnMain { calls.calls.count == 1 }
    #expect(model.isSubmitting)
    gate.open()
    #expect(await task.value == true)
    #expect(calls.calls == ["space"])
    #expect(holder.submittingDuringCreate == true)
    #expect(model.isSubmitting == false)
  }

  @Test func submitReportsCreateFailure() async {
    let model = SpaceCreateModel(create: { _ in false })
    #expect(await model.submit() == false)
  }

  @Test func secondSubmitIsIgnoredWhileInFlight() async throws {
    let gate = Gate()
    let calls = CallRecorder()
    let model = SpaceCreateModel(create: { name in
      calls.record(name)
      await gate.wait()
      return true
    })
    model.name.value = "space"
    let task = Task { await model.submit() }
    try await waitOnMain { calls.calls.count == 1 }
    #expect(await model.submit() == nil)
    #expect(calls.calls == ["space"])
    gate.open()
    #expect(await task.value == true)
  }

  @Test func focusNameRequestsFieldFocus() {
    let model = SpaceCreateModel(create: { _ in true })
    #expect(model.name.focusRequest == 0)
    model.focusName()
    #expect(model.name.focusRequest == 1)
  }

  @Test func submitEndsEditingBeforeCreate() async throws {
    let holder = Holder()
    let model = SpaceCreateModel(create: { _ in
      holder.resignRequestDuringCreate = holder.model?.name.resignRequest
      return true
    })
    holder.model = model
    model.name.isFocused = true
    #expect(await model.submit() == true)
    #expect(holder.resignRequestDuringCreate == 1)
  }
}
