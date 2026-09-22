import Core
import Foundation

struct RecentGroup: Equatable, Sendable, Identifiable {
  let id: String
  let label: String
  let documents: [RecentDocument]
}

enum RecentGrouping {
  static func groups(_ documents: [RecentDocument], now: Date) -> [RecentGroup] {
    let today = KSTDay(now)
    var groups: [RecentGroup] = []
    for document in documents {
      guard let viewedAt = document.viewedAt else { continue }
      let bucket = bucket(for: KSTDay(viewedAt), today: today)
      if let last = groups.last, last.id == bucket.id {
        groups[groups.count - 1] = RecentGroup(
          id: last.id, label: last.label, documents: last.documents + [document])
      } else {
        groups.append(RecentGroup(id: bucket.id, label: bucket.label, documents: [document]))
      }
    }
    return groups
  }

  private static func bucket(for day: KSTDay, today: KSTDay) -> (id: String, label: String) {
    if day >= today { return ("today", "오늘") }
    if day >= today.adding(days: -1) { return ("yesterday", "어제") }
    if day >= today.adding(days: -7) { return ("week", "지난 7일") }
    if day >= today.adding(days: -30) { return ("month", "지난 30일") }
    let id = String(format: "%04d-%02d", day.year, day.month)
    return (id, day.year == today.year ? "\(day.month)월" : "\(day.year)년 \(day.month)월")
  }
}
