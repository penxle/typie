import CoreGraphics
import Testing

@testable import Editor

@MainActor
@Suite struct EditorScrollbarModelTests {
  private func layout(_ visible: Double, _ content: Double, _ position: Double)
    -> EditorScrollbarLayout
  {
    EditorScrollbarLayout(visibleHeight: visible, contentHeight: content, scrollPosition: position)
  }

  private func decide(
    canGrab: Bool = true, _ dx: Double, _ dy: Double, slop: Double = 8, after milliseconds: Int
  ) -> EditorScrollbarGrab {
    EditorScrollbarGrab.decide(
      canGrab: canGrab, displacement: CGVector(dx: dx, dy: dy), slop: slop,
      elapsed: .milliseconds(milliseconds))
  }

  @Test func theBarIsAbsentWhenTheTrackCannotFitTheShortestThumb() {
    let bar = layout(33, 1800, 240)
    #expect(!bar.isVisible)
    #expect(bar.trackLength == 29)
    #expect(bar.thumbLength == 0)
    #expect(bar.thumbOffset == 0)
  }

  @Test func aTrackExactlyAsLongAsTheShortestThumbIsFilledByIt() {
    let bar = layout(34, 1800, 240)
    #expect(bar.isVisible)
    #expect(bar.trackLength == 30)
    #expect(bar.thumbLength == 30)
    #expect(bar.thumbOffset == 0)
  }

  @Test func theThumbIsTheVisibleShareOfTheTrack() {
    let bar = layout(800, 3200, 1200)
    #expect(bar.isVisible)
    #expect(bar.trackLength == 796)
    #expect(bar.thumbLength == 199)
    #expect(bar.thumbOffset == 298.5)
    #expect(bar.maxScroll == 2400)
  }

  @Test func aLongDocumentKeepsTheThumbAtTheShortestLength() {
    let bar = layout(800, 100000, 0)
    #expect(bar.isVisible)
    #expect(bar.thumbLength == 30)
  }

  @Test func contentThatFitsHasNoBar() {
    for content in [0.0, 400, 800] {
      #expect(!layout(800, content, 0).isVisible)
    }
    #expect(layout(800, 801, 0).isVisible)
    #expect(!layout(0, 100, 0).isVisible)
  }

  @Test func theThumbReachesBothEndsOfTheTrack() {
    #expect(layout(800, 3200, 0).thumbOffset == 0)
    #expect(layout(800, 3200, 2400).thumbOffset == 597)
  }

  @Test func overscrollKeepsTheThumbAtTheEnds() {
    #expect(layout(800, 3200, -80).thumbOffset == 0)
    #expect(layout(800, 3200, 2500).thumbOffset == 597)
  }

  @Test func theThumbSitsTwoPointsInsideTheRightEdgeAndGrowsInward() {
    let bar = layout(800, 3200, 1200)
    #expect(
      bar.thumbFrame(width: 44, thickness: 6) == CGRect(x: 36, y: 300.5, width: 6, height: 199))
    #expect(
      bar.thumbFrame(width: 44, thickness: 10) == CGRect(x: 32, y: 300.5, width: 10, height: 199))
  }

  @Test func aShortThumbIsTouchedThroughAMinimumAreaCenteredOnIt() {
    #expect(
      layout(800, 100000, 0).touchArea(width: 44, thickness: 6)
        == CGRect(x: 0, y: -5, width: 44, height: 44))
  }

  @Test func aLongThumbIsTouchedAlongItsWholeLength() {
    #expect(
      layout(800, 3200, 1200).touchArea(width: 44, thickness: 6)
        == CGRect(x: 0, y: 300.5, width: 44, height: 199))
  }

  @Test func theTouchAreaStaysInsideTheRightEdge() {
    let area = layout(800, 3200, 1200).touchArea(width: 200, thickness: 6)
    #expect(area.minX == 156)
    #expect(area.maxX == 200)
  }

  @Test func aStillPressWaitsBeforeTheHoldDuration() {
    #expect(decide(0, 0, after: 299) == .pending)
  }

  @Test func movementWithinTheSlopWaitsBeforeTheHoldDuration() {
    #expect(decide(2, 4, after: 299) == .pending)
  }

  @Test func movementExactlyAtTheSlopWaits() {
    #expect(decide(0, 8, after: 299) == .pending)
  }

  @Test func aStillPressGrabsAtTheHoldDuration() {
    #expect(decide(0, 0, after: 300) == .claim)
  }

  @Test func movementWithinTheSlopGrabsAtTheHoldDuration() {
    #expect(decide(2, 4, after: 300) == .claim)
  }

  @Test func verticalMovementPastTheSlopGrabsRightAway() {
    #expect(decide(2, 12, after: 0) == .claim)
  }

  @Test func sidewaysMovementPastTheSlopYields() {
    #expect(decide(12, 2, after: 0) == .yield)
  }

  @Test func sidewaysMovementPastTheSlopYieldsEvenAtTheHoldDuration() {
    #expect(decide(12, 2, after: 300) == .yield)
  }

  @Test func diagonalMovementPastTheSlopYields() {
    #expect(decide(8, 8, after: 0) == .yield)
  }

  @Test func aStillPressOnAHiddenBarYieldsAtTheHoldDuration() {
    #expect(decide(canGrab: false, 0, 0, after: 300) == .yield)
  }

  @Test func verticalMovementOnAHiddenBarYields() {
    #expect(decide(canGrab: false, 2, 12, after: 0) == .yield)
  }

  @Test func theDefaultSlopIsTenPoints() {
    let still = EditorScrollbarGrab.decide(
      canGrab: true, displacement: CGVector(dx: 0, dy: 10), elapsed: .zero)
    let moved = EditorScrollbarGrab.decide(
      canGrab: true, displacement: CGVector(dx: 0, dy: 10.01), elapsed: .zero)
    #expect(still == .pending)
    #expect(moved == .claim)
  }

  @Test func draggingMovesTheDocumentInProportionToTheThumbTravel() throws {
    var drag = EditorScrollbarDrag(layout(100, 200, 1))
    let moved = drag.move(by: 20, in: layout(100, 200, 1))
    let position = try #require(moved)
    #expect(abs(position - 42.666667) < 0.0001)
  }

  @Test func aChangedExtentRestartsTheDragFromTheCurrentPosition() throws {
    var drag = EditorScrollbarDrag(layout(100, 200, 1))
    let firstMove = drag.move(by: 20, in: layout(100, 200, 1))
    let first = try #require(firstMove)
    let secondMove = drag.move(by: 10, in: layout(100, 300, first))
    let second = try #require(secondMove)
    #expect(abs(second - 73.916667) < 0.0001)
  }

  @Test func theDocumentFollowsTheFingerBackFromBeyondTheEnd() throws {
    var drag = EditorScrollbarDrag(layout(100, 200, 50))
    var positions: [Double?] = []
    for delta in [100.0, -10, -60] {
      positions.append(
        drag.move(by: delta, in: layout(100, 200, positions.last.flatMap { $0 } ?? 50)))
    }
    #expect(positions == [100, 100, 100])
    let moved = drag.move(by: -10, in: layout(100, 200, 100))
    let back = try #require(moved)
    #expect(abs(back - 91.666667) < 0.0001)
  }

  @Test func draggingPastTheTopStopsAtTheTop() {
    var drag = EditorScrollbarDrag(layout(100, 200, 1))
    let position = drag.move(by: -20, in: layout(100, 200, 1))
    #expect(position == 0)
  }

  @Test func aMoveWithoutVerticalDistanceLeavesTheDocumentAlone() {
    var drag = EditorScrollbarDrag(layout(100, 200, 30))
    let position = drag.move(by: 0, in: layout(100, 200, 30))
    #expect(position == nil)
  }

  @Test func theBarStartsHidden() {
    let model = EditorScrollbarModel(sleep: parkedSleep)
    #expect(!model.canGrab)
    #expect(model.opacity == 0)
  }

  @Test func aUserScrollShowsTheBar() {
    let model = EditorScrollbarModel(sleep: parkedSleep)
    var changes = 0
    model.onChange = { changes += 1 }
    model.scrolled(automatic: false)

    #expect(model.canGrab)
    #expect(model.opacity == 1)
    #expect(model.thumbOpacity == 0.5)
    #expect(model.thickness == 6)
    #expect(changes == 1)
    model.hide()
  }

  @Test func anAppScrollShowsAFainterBar() {
    let model = EditorScrollbarModel(sleep: parkedSleep)
    model.scrolled(automatic: true)

    #expect(model.canGrab)
    #expect(model.opacity == 0.65)
    #expect(model.thumbOpacity == 0.22)
    #expect(model.thickness == 6)
    model.hide()
  }

  @Test func theBarStaysForOneAndAHalfSecondsAfterTheLastScroll() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = EditorScrollbarModel(now: { clock.now }, sleep: gate.sleep)
    model.scrolled(automatic: false)
    try await waitOnMain { gate.isWaiting(.milliseconds(1500)) }

    clock.advance(.milliseconds(1499))
    gate.release(.milliseconds(1500))
    try await waitOnMain { gate.isWaiting(.milliseconds(1)) }
    #expect(model.canGrab)

    clock.advance(.milliseconds(1))
    gate.release(.milliseconds(1))
    try await waitOnMain { !model.canGrab }
    #expect(model.opacity == 0)
    #expect(gate.requests == [.milliseconds(1500), .milliseconds(1)])
  }

  @Test func laterScrollsPushTheHideBack() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = EditorScrollbarModel(now: { clock.now }, sleep: gate.sleep)
    model.scrolled(automatic: false)
    try await waitOnMain { gate.isWaiting(.milliseconds(1500)) }

    clock.advance(.milliseconds(1000))
    model.scrolled(automatic: true)
    clock.advance(.milliseconds(500))
    gate.release(.milliseconds(1500))
    try await waitOnMain { gate.isWaiting(.milliseconds(1000)) }
    #expect(model.canGrab)
    #expect(model.opacity == 0.65)

    clock.advance(.milliseconds(1000))
    gate.release(.milliseconds(1000))
    try await waitOnMain { !model.canGrab }
    #expect(gate.requests == [.milliseconds(1500), .milliseconds(1000)])
  }

  @Test func holdingTheThumbKeepsTheBarUp() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = EditorScrollbarModel(now: { clock.now }, sleep: gate.sleep)
    model.scrolled(automatic: false)
    try await waitOnMain { gate.isWaiting(.milliseconds(1500)) }

    model.grabChanged(true)
    model.scrolled(automatic: false)
    clock.advance(.seconds(3))
    gate.release(.milliseconds(1500))
    try await Task.sleep(for: .milliseconds(50))

    #expect(model.canGrab)
    #expect(model.opacity == 1)
    #expect(model.thumbOpacity == 0.8)
    #expect(model.thickness == 10)
    #expect(gate.requests == [.milliseconds(1500)])
  }

  @Test func releasingTheThumbStartsTheUsualDelay() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = EditorScrollbarModel(now: { clock.now }, sleep: gate.sleep)
    model.grabChanged(true)
    clock.advance(.seconds(3))
    model.grabChanged(false)
    #expect(model.thickness == 6)
    #expect(model.thumbOpacity == 0.5)
    try await waitOnMain { gate.isWaiting(.milliseconds(1500)) }

    clock.advance(.milliseconds(1499))
    gate.release(.milliseconds(1500))
    try await waitOnMain { gate.isWaiting(.milliseconds(1)) }
    #expect(model.canGrab)

    clock.advance(.milliseconds(1))
    gate.release(.milliseconds(1))
    try await waitOnMain { !model.canGrab }
    #expect(gate.requests == [.milliseconds(1500), .milliseconds(1)])
  }

  @Test func grabbingAfterAnAppScrollShowsTheUserBar() {
    let model = EditorScrollbarModel(sleep: parkedSleep)
    model.scrolled(automatic: true)
    model.grabChanged(true)

    #expect(model.opacity == 1)
    #expect(model.thumbOpacity == 0.8)
    model.grabChanged(false)
    model.hide()
  }

  @Test func anAppScrollWhileHoldingKeepsTheBarAtTheFainterLevel() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = EditorScrollbarModel(now: { clock.now }, sleep: gate.sleep)
    model.grabChanged(true)
    model.scrolled(automatic: true)
    try await Task.sleep(for: .milliseconds(50))

    #expect(model.canGrab)
    #expect(model.opacity == 0.65)
    #expect(model.thumbOpacity == 0.45)
    #expect(model.thickness == 10)
    #expect(gate.requests.isEmpty)
    gate.releaseAll()
  }

  @Test func repeatedGrabStatesChangeNothing() {
    let model = EditorScrollbarModel(sleep: parkedSleep)
    var changes = 0
    model.onChange = { changes += 1 }
    model.grabChanged(false)
    #expect(!model.canGrab)

    model.grabChanged(true)
    model.grabChanged(true)
    #expect(changes == 1)
  }

  @Test func hidingClearsTheBarAtOnce() async throws {
    let clock = TestClock()
    let gate = SleepGate(strict: true)
    let model = EditorScrollbarModel(now: { clock.now }, sleep: gate.sleep)
    model.scrolled(automatic: true)
    try await waitOnMain { gate.isWaiting(.milliseconds(1500)) }

    model.hide()
    #expect(!model.canGrab)
    #expect(model.opacity == 0)

    clock.advance(.milliseconds(1500))
    gate.release(.milliseconds(1500))
    model.scrolled(automatic: false)
    try await waitOnMain { gate.waitingCount(.milliseconds(1500)) == 1 }
    try await Task.sleep(for: .milliseconds(50))
    #expect(model.canGrab)
    model.hide()
    gate.releaseAll()
  }
}
