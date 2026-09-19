import Observation

@Observable
@MainActor
public final class TThemeSettings {
  public var mode: TThemeMode

  public init(mode: TThemeMode = .system) {
    self.mode = mode
  }
}
