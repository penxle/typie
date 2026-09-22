import GraphQL

enum RecentSort: String, CaseIterable, Sendable, Identifiable {
  case viewed
  case updated

  var id: String { rawValue }

  var graphQL: GraphQLEnum<GraphQL.RecentDocumentSort> {
    switch self {
    case .viewed: .case(.viewedAt)
    case .updated: .case(.updatedAt)
    }
  }

  var title: String {
    switch self {
    case .viewed: "최근 본 순서"
    case .updated: "최근 수정한 순서"
    }
  }

  var shortTitle: String {
    switch self {
    case .viewed: "본 순서"
    case .updated: "수정한 순서"
    }
  }

  var next: RecentSort {
    switch self {
    case .viewed: .updated
    case .updated: .viewed
    }
  }

  var emptyText: String {
    switch self {
    case .viewed: "최근 본 문서가 없어요"
    case .updated: "최근 수정한 문서가 없어요"
    }
  }
}
