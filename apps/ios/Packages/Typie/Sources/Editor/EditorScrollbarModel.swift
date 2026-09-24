import CoreGraphics

struct EditorScrollbarLayout: Equatable {
  static let minimumThumbLength: Double = 30
  static let edgeInset: Double = 2
  static let touchExtent: Double = 44

  let visibleHeight: Double
  let contentHeight: Double
  let scrollPosition: Double
  let isVisible: Bool
  let trackLength: Double
  let thumbLength: Double
  let thumbOffset: Double

  init(visibleHeight: Double = 0, contentHeight: Double = 0, scrollPosition: Double = 0) {
    self.visibleHeight = visibleHeight
    self.contentHeight = contentHeight
    self.scrollPosition = scrollPosition
    let track = max(0, visibleHeight - 2 * Self.edgeInset)
    let visible =
      visibleHeight > 0 && contentHeight > visibleHeight && track >= Self.minimumThumbLength
    let thumb =
      visible ? min(max(track * visibleHeight / contentHeight, Self.minimumThumbLength), track) : 0
    let travel = max(0, track - thumb)
    let maxScroll = max(0, contentHeight - visibleHeight)
    isVisible = visible
    trackLength = track
    thumbLength = thumb
    thumbOffset =
      visible && maxScroll > 0 && travel > 0
      ? travel * min(max(scrollPosition / maxScroll, 0), 1) : 0
  }

  var maxScroll: Double { max(0, contentHeight - visibleHeight) }
  var thumbTravel: Double { max(0, trackLength - thumbLength) }

  func thumbFrame(width: Double, thickness: Double) -> CGRect {
    CGRect(
      x: width - Self.edgeInset - thickness, y: Self.edgeInset + thumbOffset, width: thickness,
      height: thumbLength)
  }

  func touchArea(width: Double, thickness: Double) -> CGRect {
    let thumb = thumbFrame(width: width, thickness: thickness)
    let height = max(thumbLength, Self.touchExtent)
    let x = max(0, min(thumb.midX - Self.touchExtent / 2, width - Self.touchExtent))
    return CGRect(x: x, y: thumb.midY - height / 2, width: Self.touchExtent, height: height)
  }
}

enum EditorScrollbarGrab: Equatable {
  case pending
  case claim
  case yield

  static let touchSlop: Double = 10
  static let holdDuration = Duration.milliseconds(300)

  static func decide(
    canGrab: Bool, displacement: CGVector, slop: Double = touchSlop, elapsed: Duration
  ) -> EditorScrollbarGrab {
    let slop = max(0, slop)
    if displacement.dx * displacement.dx + displacement.dy * displacement.dy > slop * slop {
      guard abs(displacement.dy) > abs(displacement.dx) else { return .yield }
      return canGrab ? .claim : .yield
    }
    guard elapsed >= holdDuration else { return .pending }
    return canGrab ? .claim : .yield
  }
}

struct EditorScrollbarDrag {
  private struct Extent: Equatable {
    var track: Double
    var thumb: Double
    var visible: Double
    var content: Double

    init(_ layout: EditorScrollbarLayout) {
      track = layout.trackLength
      thumb = layout.thumbLength
      visible = layout.visibleHeight
      content = layout.contentHeight
    }
  }

  private var extent: Extent
  private var start: Double
  private var distance: Double = 0

  init(_ layout: EditorScrollbarLayout) {
    extent = Extent(layout)
    start = layout.scrollPosition
  }

  mutating func move(by delta: Double, in layout: EditorScrollbarLayout) -> Double? {
    guard delta != 0 else { return nil }
    let latest = Extent(layout)
    if latest != extent {
      extent = latest
      start = layout.scrollPosition
      distance = 0
    }
    distance += delta
    let maxScroll = layout.maxScroll
    let travel = layout.thumbTravel
    guard maxScroll > 0, travel > 0 else { return min(max(start, 0), maxScroll) }
    return min(max(start + distance * maxScroll / travel, 0), maxScroll)
  }
}

@MainActor
final class EditorScrollbarModel {
  static let dwell = Duration.milliseconds(1500)
  static let fadeDuration: Double = 0.3

  private(set) var isShown = false
  private(set) var isAutomatic = false
  private(set) var isGrabbed = false
  var onChange: (() -> Void)?

  var canGrab: Bool { isShown || isGrabbed }
  var opacity: Double { canGrab ? (isAutomatic ? 0.65 : 1) : 0 }
  var thumbOpacity: Double {
    isAutomatic ? (isGrabbed ? 0.45 : 0.22) : (isGrabbed ? 0.8 : 0.5)
  }
  var thickness: Double { isGrabbed ? 10 : 6 }

  private let countdown: HideCountdown

  init(
    now: @escaping @MainActor () -> ContinuousClock.Instant = { ContinuousClock.now },
    sleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) }
  ) {
    countdown = HideCountdown(delay: Self.dwell, now: now, sleep: sleep)
    countdown.onExpire = { [weak self] in self?.expired() }
  }

  func scrolled(automatic: Bool) {
    isShown = true
    isAutomatic = automatic
    if !isGrabbed {
      countdown.start()
    }
    onChange?()
  }

  func grabChanged(_ grabbed: Bool) {
    guard grabbed != isGrabbed else { return }
    isGrabbed = grabbed
    isShown = true
    isAutomatic = false
    if grabbed {
      countdown.stop()
    } else {
      countdown.start()
    }
    onChange?()
  }

  func hide() {
    countdown.stop()
    isShown = false
    isAutomatic = false
    onChange?()
  }

  private func expired() {
    isShown = false
    onChange?()
  }
}
