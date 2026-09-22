enum Route: Hashable, Sendable {
  case home
  case notes
  case prism
  case square
  case settings
  case siteSwitcher
  case userGoal
  case profile
  case pinnedEntities
  case recentDocuments
  case siteEntities
  case folder(entityId: String)
  case folderDetails(entityId: String)
  case document(entityId: String)
  case documentBodySettings(entityId: String)
}

enum MainTab: String, CaseIterable, Sendable {
  case home
  case notes
  case prism
  case square

  static let initial = MainTab.home

  var index: Int {
    Self.allCases.firstIndex(of: self)!
  }

  var route: Route {
    switch self {
    case .home: .home
    case .notes: .notes
    case .prism: .prism
    case .square: .square
    }
  }
}
