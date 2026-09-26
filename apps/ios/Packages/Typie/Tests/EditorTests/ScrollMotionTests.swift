import Testing

@testable import Editor

@Suite struct ScrollMotionTests {
  @Test func aDecelerationHoldsItsDestinationUntilItStops() {
    #expect(
      trace([
        { $0.willBeginDragging() },
        { $0.willEndDragging(target: 4000) },
        { $0.didEndDragging(willDecelerate: true) },
        { $0.didEndDecelerating() },
      ]) == [Step(false, nil), Step(true, 4000), Step(false, 4000), Step(true, nil)])
  }

  @Test func releasingWithoutDecelerationClearsTheDestination() {
    #expect(
      trace([
        { $0.willEndDragging(target: 300) },
        { $0.didEndDragging(willDecelerate: false) },
      ]) == [Step(true, 300), Step(true, nil)])
  }

  @Test func draggingAgainDuringADecelerationClearsTheDestination() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.didEndDragging(willDecelerate: true) },
        { $0.willBeginDragging() },
        { $0.didEndDecelerating() },
      ]) == [Step(true, 4000), Step(false, 4000), Step(true, nil), Step(false, nil)])
  }

  @Test func scrollingToTheTopHoldsTheTopUntilItArrives() {
    #expect(
      trace([
        { $0.willScrollToTop(top: -116, current: 3000, pixel: 0.5) },
        { $0.didScrollToTop() },
      ]) == [Step(true, -116), Step(true, nil)])
  }

  @Test func catchingTheSameDestinationAgainIsNoChange() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.willEndDragging(target: 4000) },
        { $0.willScrollToTop(top: 4000, current: 3000, pixel: 0.5) },
        { $0.willEndDragging(target: 4100) },
      ]) == [Step(true, 4000), Step(false, 4000), Step(false, 4000), Step(true, 4100)])
  }

  @Test func clearingWithoutADestinationIsNoChange() {
    #expect(
      trace([
        { $0.willBeginDragging() },
        { $0.didEndDragging(willDecelerate: false) },
        { $0.didEndDecelerating() },
        { $0.didScrollToTop() },
        { $0.reset() },
      ]) == Array(repeating: Step(false, nil), count: 5))
  }

  @Test func resetClearsTheDestination() {
    #expect(
      trace([
        { $0.willScrollToTop(top: -116, current: 3000, pixel: 0.5) },
        { $0.reset() },
      ]) == [Step(true, -116), Step(true, nil)])
  }

  @Test func theEndOfADecelerationLeavesAScrollToTopInPlace() {
    #expect(
      trace([
        { $0.willScrollToTop(top: -116, current: 3000, pixel: 0.5) },
        { $0.didEndDecelerating() },
        { $0.didEndDragging(willDecelerate: false) },
      ]) == [Step(true, -116), Step(false, -116), Step(false, -116)])
  }

  @Test func arrivingAtTheTopLeavesAFlingInPlace() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.didEndDragging(willDecelerate: true) },
        { $0.didScrollToTop() },
      ]) == [Step(true, 4000), Step(false, 4000), Step(false, 4000)])
  }

  @Test func tappingTheStatusBarDuringAFlingHeadsForTheTop() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.didEndDragging(willDecelerate: true) },
        { $0.validate(isDecelerating: true, isTracking: false) },
        { $0.willScrollToTop(top: -116, current: 3000, pixel: 0.5) },
        { $0.didEndDecelerating() },
        { $0.validate(isDecelerating: false, isTracking: false) },
        { $0.validate(isDecelerating: false, isTracking: false) },
        { $0.didScrollToTop() },
      ]) == [
        Step(true, 4000), Step(false, 4000), Step(false, 4000), Step(true, -116),
        Step(false, -116), Step(false, -116), Step(false, -116), Step(true, nil),
      ])
  }

  @Test func tappingTheStatusBarDuringAFlingThatEndsAtTheTopStillWaitsForTheTop() {
    #expect(
      trace([
        { $0.willEndDragging(target: -116) },
        { $0.didEndDragging(willDecelerate: true) },
        { $0.willScrollToTop(top: -116, current: 3000, pixel: 0.5) },
        { $0.didEndDecelerating() },
        { $0.didScrollToTop() },
      ]) == [
        Step(true, -116), Step(false, -116), Step(false, -116), Step(false, -116),
        Step(true, nil),
      ])
  }

  @Test func scrollingToTheTopFromTheTopHoldsNothing() {
    #expect(
      trace([
        { $0.willScrollToTop(top: -116, current: -116, pixel: 0.5) },
        { $0.willScrollToTop(top: -116, current: -115.6, pixel: 0.5) },
        { $0.willScrollToTop(top: -116, current: -116.4, pixel: 0.5) },
        { $0.didScrollToTop() },
      ]) == Array(repeating: Step(false, nil), count: 4))
  }

  @Test func scrollingToTheTopFromTheTopLeavesAFlingAlone() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.didEndDragging(willDecelerate: true) },
        { $0.willScrollToTop(top: -116, current: -115.8, pixel: 0.5) },
        { $0.didScrollToTop() },
        { $0.didEndDecelerating() },
      ]) == [
        Step(true, 4000), Step(false, 4000), Step(false, 4000), Step(false, 4000),
        Step(true, nil),
      ])
    #expect(
      trace([
        { $0.willEndDragging(target: -116) },
        { $0.didEndDragging(willDecelerate: true) },
        { $0.willScrollToTop(top: -116, current: -116.2, pixel: 0.5) },
        { $0.didEndDecelerating() },
      ]) == [Step(true, -116), Step(false, -116), Step(false, -116), Step(true, nil)])
  }

  @Test func scrollingToTheTopFromAPixelAwayHeadsForTheTop() {
    #expect(
      trace([
        { $0.willScrollToTop(top: -116, current: -115.5, pixel: 0.5) }
      ]) == [Step(true, -116)])
    #expect(
      trace([
        { $0.willScrollToTop(top: -116, current: -115.4, pixel: 0.5) }
      ]) == [Step(true, -116)])
    #expect(
      trace([
        { $0.willScrollToTop(top: -116, current: -116.6, pixel: 0.5) }
      ]) == [Step(true, -116)])
  }

  @Test func aFingerOnAStoppedFlingClearsIt() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.validate(isDecelerating: true, isTracking: false) },
        { $0.validate(isDecelerating: false, isTracking: true) },
      ]) == [Step(true, 4000), Step(false, 4000), Step(true, nil)])
  }

  @Test func draggingClearsAScrollToTop() {
    #expect(
      trace([
        { $0.willScrollToTop(top: -116, current: 3000, pixel: 0.5) },
        { $0.willBeginDragging() },
      ]) == [Step(true, -116), Step(true, nil)])
  }

  @Test func resetClearsAFling() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.reset() },
      ]) == [Step(true, 4000), Step(true, nil)])
  }

  @Test func aFlingStaysWhileTheScrollViewDecelerates() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.validate(isDecelerating: true, isTracking: false) },
        { $0.validate(isDecelerating: true, isTracking: true) },
        { $0.validate(isDecelerating: false, isTracking: false) },
      ]) == [Step(true, 4000), Step(false, 4000), Step(false, 4000), Step(true, nil)])
  }

  @Test func aFlingOutlivesOneTickBeforeTheDecelerationShows() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.validate(isDecelerating: false, isTracking: false) },
        { $0.validate(isDecelerating: true, isTracking: false) },
      ]) == [Step(true, 4000), Step(false, 4000), Step(false, 4000)])
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.validate(isDecelerating: false, isTracking: false) },
        { $0.validate(isDecelerating: false, isTracking: false) },
      ]) == [Step(true, 4000), Step(false, 4000), Step(true, nil)])
  }

  @Test func everyNewFlingGetsItsOwnFirstTick() {
    #expect(
      trace([
        { $0.willEndDragging(target: 4000) },
        { $0.validate(isDecelerating: true, isTracking: false) },
        { $0.willBeginDragging() },
        { $0.willEndDragging(target: 9000) },
        { $0.validate(isDecelerating: false, isTracking: false) },
      ]) == [
        Step(true, 4000), Step(false, 4000), Step(true, nil), Step(true, 9000),
        Step(false, 9000),
      ])
  }

  @Test func aScrollToTopStaysUntilAFingerLands() {
    #expect(
      trace([
        { $0.willScrollToTop(top: -116, current: 3000, pixel: 0.5) },
        { $0.validate(isDecelerating: false, isTracking: false) },
        { $0.validate(isDecelerating: true, isTracking: false) },
        { $0.validate(isDecelerating: false, isTracking: true) },
      ]) == [Step(true, -116), Step(false, -116), Step(false, -116), Step(true, nil)])
  }

  @Test func validatingWithoutADestinationIsNoChange() {
    #expect(
      trace([
        { $0.validate(isDecelerating: false, isTracking: false) },
        { $0.validate(isDecelerating: false, isTracking: true) },
        { $0.validate(isDecelerating: true, isTracking: false) },
      ]) == Array(repeating: Step(false, nil), count: 3))
  }

  private struct Step: Equatable {
    var changed: Bool
    var destination: Double?

    init(_ changed: Bool, _ destination: Double?) {
      self.changed = changed
      self.destination = destination
    }
  }

  private func trace(_ events: [(inout ScrollMotion) -> Bool]) -> [Step] {
    var motion = ScrollMotion()
    return events.map { event in
      let changed = event(&motion)
      return Step(changed, motion.destination)
    }
  }
}
