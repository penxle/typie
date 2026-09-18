public enum ThemeMode: String, Codable, CaseIterable, Sendable {
  case system
  case light
  case dark
}

public enum ResolvedThemeMode: Sendable {
  case light
  case dark
}
