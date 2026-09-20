public enum TThemeMode: String, Codable, CaseIterable, Sendable {
  case system
  case light
  case dark
}

public enum TResolvedThemeMode: Sendable {
  case light
  case dark
}
