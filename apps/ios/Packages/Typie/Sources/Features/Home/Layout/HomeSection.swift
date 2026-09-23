enum HomeSection: String, CaseIterable, Sendable, Identifiable {
  case goal
  case pinned
  case recent
  case all

  var id: String { rawValue }

  var title: String {
    switch self {
    case .goal: "목표"
    case .pinned: "고정"
    case .recent: "최근"
    case .all: "전체"
    }
  }

  var canHide: Bool { self != .all }
}
