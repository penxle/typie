public enum Route: Hashable, Sendable {
  case home
  case studio
  case notes
  case settings
  case folder(entityId: String)
  case folderDetails(entityId: String)
  case document(entityId: String)
  case documentBodySettings(entityId: String)
}

public enum MainTab: CaseIterable, Sendable {
  case home
  case studio
  case notes

  public var route: Route {
    switch self {
    case .home: .home
    case .studio: .studio
    case .notes: .notes
    }
  }
}
