enum Route: Hashable, Sendable {
  case home
  case studio
  case notes
  case settings
  case siteSwitcher
  case userGoal
  case pinnedEntities
  case studioTree
  case folder(entityId: String)
  case folderDetails(entityId: String)
  case document(entityId: String)
  case documentBodySettings(entityId: String)
}

enum MainTab: String, CaseIterable, Sendable {
  case home
  case studio
  case notes

  static let initial = MainTab.home

  var index: Int {
    Self.allCases.firstIndex(of: self)!
  }

  var route: Route {
    switch self {
    case .home: .home
    case .studio: .studio
    case .notes: .notes
    }
  }
}
