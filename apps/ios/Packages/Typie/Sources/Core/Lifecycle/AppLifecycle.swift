import Observation

@MainActor @Observable
public final class AppLifecycle {
  public enum State: Sendable, Equatable {
    case foreground
    case background
  }

  public private(set) var state: State = .background
  public private(set) var foregroundGeneration = 0

  @ObservationIgnored private var hasBeenForeground = false

  public init() {}

  public func update(foreground: Bool) {
    let next: State = foreground ? .foreground : .background
    guard next != state else { return }
    if next == .foreground, hasBeenForeground {
      foregroundGeneration += 1
    }
    state = next
    if next == .foreground {
      hasBeenForeground = true
    }
  }
}
