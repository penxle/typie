import EditorFFI
import Testing

@testable import Editor

@MainActor
@Suite struct PositionIndicatorModelTests {
  private let paginated = FrameLayout.paginated(
    marginTop: 94, marginBottom: 94, marginLeft: 94, marginRight: 94)

  @Test func pageModeShowsTheCurrentPageOverThePageCount() {
    #expect(
      PositionIndicatorModel.text(
        position: FramePosition(page: 3, pages: 215, percent: 1), layout: paginated) == "3/215")
  }

  @Test func continuousModeShowsThePercentage() {
    #expect(
      PositionIndicatorModel.text(
        position: FramePosition(page: 2, pages: 267, percent: 50), layout: .continuous) == "50%")
  }

  @Test func documentsWithoutPagesShowNothing() {
    #expect(PositionIndicatorModel.text(position: nil, layout: .continuous) == nil)
  }

  @Test func userScrollShowsTheValue() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    var changes = 0
    model.onChange = { changes += 1 }
    model.userScrolled("50%")

    #expect(model.isVisible)
    #expect(model.text == "50%")
    #expect(changes == 1)
    model.autoScrolled("50%")
  }

  @Test func theValueStaysForThreeHundredMillisecondsAfterTheLastUserScroll() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = PositionIndicatorModel(now: { clock.now }, sleep: gate.sleep)
    model.userScrolled("12/215")
    try await waitOnMain { gate.isWaiting(.milliseconds(300)) }

    clock.advance(.milliseconds(299))
    gate.release(.milliseconds(300))
    try await waitOnMain { gate.isWaiting(.milliseconds(1)) }
    #expect(model.isVisible)

    clock.advance(.milliseconds(1))
    gate.release(.milliseconds(1))
    try await waitOnMain { !model.isVisible }
    #expect(model.text == "12/215")
    #expect(gate.requests == [.milliseconds(300), .milliseconds(1)])
  }

  @Test func laterUserScrollsPushTheHideBack() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = PositionIndicatorModel(now: { clock.now }, sleep: gate.sleep)
    model.userScrolled("10%")
    try await waitOnMain { gate.isWaiting(.milliseconds(300)) }

    clock.advance(.milliseconds(200))
    model.userScrolled("11%")
    clock.advance(.milliseconds(100))
    gate.release(.milliseconds(300))
    try await waitOnMain { gate.isWaiting(.milliseconds(200)) }
    #expect(model.isVisible)
    #expect(model.text == "11%")

    clock.advance(.milliseconds(200))
    gate.release(.milliseconds(200))
    try await waitOnMain { !model.isVisible }
  }

  @Test func automaticScrollNeverShowsItsValue() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    var seen: [String] = []
    model.onChange = { seen.append("\(model.text ?? "-") \(model.isVisible)") }
    model.userScrolled("50%")
    model.autoScrolled("55%")
    model.autoScrolled("60%")

    #expect(!model.isVisible)
    #expect(model.text == "60%")
    #expect(seen == ["50% true", "55% false", "60% false"])
  }

  @Test func automaticScrollAloneShowsNothing() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    var shown: [Bool] = []
    model.onChange = { shown.append(model.isVisible) }
    model.autoScrolled("50%")
    model.autoScrolled("51%")

    #expect(!model.isVisible)
    #expect(model.text == "51%")
    #expect(shown == [false, false])
  }

  @Test func contentThatCannotScrollShowsNothing() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    model.userScrolled("1/1")
    model.userScrolled(nil)

    #expect(!model.isVisible)
    model.autoScrolled(nil)
  }

  @Test func scrubbingHoldsTheValueUpWhileTheFingerRests() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = PositionIndicatorModel(now: { clock.now }, sleep: gate.sleep)
    model.userScrolled("12/215")
    try await waitOnMain { gate.isWaiting(.milliseconds(300)) }

    model.scrubbingChanged(true)
    model.userScrolled("40/215")
    clock.advance(.seconds(2))
    gate.release(.milliseconds(300))
    try await Task.sleep(for: .milliseconds(50))

    #expect(model.isVisible)
    #expect(model.text == "40/215")
    #expect(gate.requests == [.milliseconds(300)])
  }

  @Test func releasingTheScrubStartsTheUsualDelay() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = PositionIndicatorModel(now: { clock.now }, sleep: gate.sleep)
    model.scrubbingChanged(true)
    model.userScrolled("3/215")
    clock.advance(.seconds(2))
    model.scrubbingChanged(false)
    try await waitOnMain { gate.isWaiting(.milliseconds(300)) }

    clock.advance(.milliseconds(299))
    gate.release(.milliseconds(300))
    try await waitOnMain { gate.isWaiting(.milliseconds(1)) }
    #expect(model.isVisible)

    clock.advance(.milliseconds(1))
    gate.release(.milliseconds(1))
    try await waitOnMain { !model.isVisible }
    #expect(model.text == "3/215")
    #expect(gate.requests == [.milliseconds(300), .milliseconds(1)])
  }

  @Test func scrubbingShowsTheLatestValueRightAway() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    model.autoScrolled("7/215")
    #expect(!model.isVisible)

    model.scrubbingChanged(true)
    #expect(model.isVisible)
    #expect(model.text == "7/215")
  }

  @Test func repeatedScrubbingStatesChangeNothing() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    var changes = 0
    model.onChange = { changes += 1 }
    model.autoScrolled("7/215")
    model.scrubbingChanged(false)
    #expect(!model.isVisible)

    model.scrubbingChanged(true)
    model.scrubbingChanged(true)
    #expect(model.isVisible)
    #expect(changes == 2)
  }

  @Test func automaticScrollHidesTheValueEvenWhileScrubbing() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    model.scrubbingChanged(true)
    model.userScrolled("5/215")
    model.autoScrolled("6/215")
    #expect(!model.isVisible)
    #expect(model.text == "6/215")

    model.userScrolled("7/215")
    #expect(model.isVisible)
  }

  @Test func onlyOneHideTimerIsEverPending() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = PositionIndicatorModel(now: { clock.now }, sleep: gate.sleep)
    model.userScrolled("1%")
    try await waitOnMain { gate.isWaiting(.milliseconds(300)) }

    for step in 2...4 {
      clock.advance(.milliseconds(50))
      model.userScrolled("\(step)%")
    }
    clock.advance(.milliseconds(150))
    gate.release(.milliseconds(300))
    try await waitOnMain { gate.isWaiting(.milliseconds(150)) }

    clock.advance(.milliseconds(150))
    gate.release(.milliseconds(150))
    try await waitOnMain { !model.isVisible }
    #expect(model.text == "4%")
    #expect(gate.requests == [.milliseconds(300), .milliseconds(150)])
  }

  @Test func aCancelledTimerWakingLateLeavesTheNewerOneAlone() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = PositionIndicatorModel(now: { clock.now }, sleep: gate.sleep)
    var changes = 0
    model.onChange = { changes += 1 }
    model.userScrolled("1/3")
    try await waitOnMain { gate.isWaiting(.milliseconds(300)) }

    model.autoScrolled("2/3")
    clock.advance(.milliseconds(100))
    model.userScrolled("3/3")
    try await waitOnMain { gate.waitingCount(.milliseconds(300)) == 2 }

    gate.release(.milliseconds(300))
    try await Task.sleep(for: .milliseconds(50))
    #expect(model.isVisible)
    #expect(gate.requests == [.milliseconds(300), .milliseconds(300)])

    clock.advance(.milliseconds(300))
    gate.release(.milliseconds(300))
    try await waitOnMain { !model.isVisible }
    #expect(model.text == "3/3")
    #expect(changes == 4)
  }

  @Test func aCancelledTimerStartingLateLeavesTheNewerOneAlone() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = PositionIndicatorModel(now: { clock.now }, sleep: gate.sleep)
    var changes = 0
    model.onChange = { changes += 1 }
    model.userScrolled("1/3")
    model.autoScrolled("2/3")
    model.userScrolled("3/3")
    clock.advance(.milliseconds(300))
    try await waitOnMain { !model.isVisible }

    #expect(model.text == "3/3")
    #expect(changes == 4)
    #expect(gate.requests.isEmpty)
  }

  @Test func aPositionChangeUpdatesTheShownValue() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    var changes = 0
    model.onChange = { changes += 1 }
    model.userScrolled("12/215")
    model.positionChanged("12/216")
    model.positionChanged("12/216")

    #expect(model.isVisible)
    #expect(model.text == "12/216")
    #expect(changes == 2)
    model.autoScrolled("12/216")
  }

  @Test func aPositionChangeUpdatesTheFadingValue() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    var seen: [String] = []
    model.onChange = { seen.append("\(model.text ?? "-") \(model.isVisible)") }
    model.userScrolled("40%")
    model.autoScrolled("45%")
    model.positionChanged("44%")

    #expect(seen == ["40% true", "45% false", "44% false"])
  }

  @Test func aPositionChangeNeverShowsAHiddenValue() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    var shown: [Bool] = []
    model.onChange = { shown.append(model.isVisible) }
    model.positionChanged("1/215")
    model.positionChanged("1/216")

    #expect(!model.isVisible)
    #expect(shown == [false, false])

    model.scrubbingChanged(true)
    #expect(model.text == "1/216")
  }

  @Test func aPositionChangeLeavesTheHideTimerAlone() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = PositionIndicatorModel(now: { clock.now }, sleep: gate.sleep)
    model.userScrolled("12/215")
    try await waitOnMain { gate.isWaiting(.milliseconds(300)) }

    clock.advance(.milliseconds(200))
    model.positionChanged("12/216")
    clock.advance(.milliseconds(100))
    gate.release(.milliseconds(300))
    try await waitOnMain { !model.isVisible }

    #expect(model.text == "12/216")
    #expect(gate.requests == [.milliseconds(300)])
  }

  @Test func aValueThatReturnsWithinTheDelayShowsAgain() {
    let model = PositionIndicatorModel(sleep: parkedSleep)
    model.userScrolled("2/2")
    model.positionChanged(nil)
    #expect(!model.isVisible)

    model.positionChanged("2/3")
    #expect(model.isVisible)
    model.autoScrolled("2/3")
  }
}
