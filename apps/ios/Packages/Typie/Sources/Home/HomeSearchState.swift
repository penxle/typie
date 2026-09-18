import Observation

@MainActor @Observable
public final class HomeSearchState {
  public static let transitionDuration: Double = 0.35
  public static let transitionBounce: Double = 0

  public var isActive = false
  public var barHeight: Double = 0
  public var titleVisible = false

  public init() {}
}
