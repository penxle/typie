internal import EditorFFI

@MainActor
final class PositionIndicatorModel {
  static let hideDelay = Duration.milliseconds(300)
  static let fadeDuration: Double = 0.3

  private(set) var text: String?
  var isVisible: Bool { text != nil && !automatic && (shown || scrubbing) }
  var onChange: (() -> Void)?

  private let countdown: HideCountdown
  private var shown = false
  private var automatic = false
  private var scrubbing = false

  init(
    now: @escaping @MainActor () -> ContinuousClock.Instant = { ContinuousClock.now },
    sleep: @escaping @Sendable (Duration) async throws -> Void = { try await Task.sleep(for: $0) }
  ) {
    countdown = HideCountdown(delay: Self.hideDelay, now: now, sleep: sleep)
    countdown.onExpire = { [weak self] in self?.expired() }
  }

  static func text(position: FramePosition?, layout: FrameLayout) -> String? {
    guard let position else { return nil }
    switch layout {
    case .paginated: return "\(position.page)/\(position.pages)"
    case .continuous: return "\(position.percent)%"
    }
  }

  func userScrolled(_ text: String?) {
    self.text = text
    shown = true
    automatic = false
    if !scrubbing {
      countdown.start()
    }
    onChange?()
  }

  func autoScrolled(_ text: String?) {
    self.text = text
    shown = false
    automatic = true
    countdown.stop()
    onChange?()
  }

  func positionChanged(_ text: String?) {
    guard text != self.text else { return }
    self.text = text
    onChange?()
  }

  func scrubbingChanged(_ isScrubbing: Bool) {
    guard isScrubbing != scrubbing else { return }
    scrubbing = isScrubbing
    shown = true
    automatic = false
    if isScrubbing {
      countdown.stop()
    } else {
      countdown.start()
    }
    onChange?()
  }

  private func expired() {
    shown = false
    onChange?()
  }
}
