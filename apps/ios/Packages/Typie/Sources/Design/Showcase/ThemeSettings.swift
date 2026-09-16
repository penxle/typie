import Observation

@Observable
@MainActor
public final class ThemeSettings {
  public var mode: ThemeMode

  public init(mode: ThemeMode = .system) {
    self.mode = mode
  }
}
