struct ScrollMotion {
  private enum Kind {
    case fling
    case top
  }

  private struct Target {
    var kind: Kind
    var offset: Double
    var ticked = false
  }

  private var target: Target?

  var destination: Double? {
    target?.offset
  }

  mutating func willBeginDragging() -> Bool {
    clear()
  }

  mutating func willEndDragging(target: Double) -> Bool {
    hold(.fling, target)
  }

  mutating func didEndDragging(willDecelerate: Bool) -> Bool {
    willDecelerate ? false : clear(.fling)
  }

  mutating func didEndDecelerating() -> Bool {
    clear(.fling)
  }

  mutating func willScrollToTop(top: Double, current: Double, pixel: Double) -> Bool {
    guard abs(current - top) >= pixel else { return false }
    return hold(.top, top)
  }

  mutating func didScrollToTop() -> Bool {
    clear(.top)
  }

  mutating func reset() -> Bool {
    clear()
  }

  mutating func validate(isDecelerating: Bool, isTracking: Bool) -> Bool {
    guard let target else { return false }
    switch target.kind {
    case .fling:
      guard isDecelerating || !target.ticked else { return clear() }
      self.target?.ticked = true
      return false
    case .top:
      return isTracking ? clear() : false
    }
  }

  private mutating func hold(_ kind: Kind, _ offset: Double) -> Bool {
    let changed = offset != destination
    if changed || target?.kind != kind {
      target = Target(kind: kind, offset: offset)
    }
    return changed
  }

  private mutating func clear(_ kind: Kind? = nil) -> Bool {
    guard let target, kind == nil || target.kind == kind else { return false }
    self.target = nil
    return true
  }
}
