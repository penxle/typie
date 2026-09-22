import Core
import Foundation
import Testing

@testable import Features

@Suite struct RecentGroupingTests {
  private let now = parseDateTime("2026-09-22T03:00:00.000Z")!

  private func document(_ id: String, _ viewedAt: String?) -> RecentDocument {
    RecentDocument(
      item: .document(
        EntityDocumentItem(
          entityId: id, icon: EntityIconSpec(kind: .document, name: "", color: ""), path: [],
          title: EntityText.documentTitle(id), updatedAt: nil)),
      viewedAt: viewedAt.flatMap(parseDateTime))
  }

  @Test func bucketsByKstDayRelativeToToday() {
    let groups = RecentGrouping.groups(
      [
        document("a", "2026-09-21T16:00:00.000Z"),
        document("b", "2026-09-21T10:00:00.000Z"),
        document("c", "2026-09-16T10:00:00.000Z"),
        document("d", "2026-09-01T10:00:00.000Z"),
        document("e", "2026-07-03T10:00:00.000Z"),
        document("f", "2025-12-25T10:00:00.000Z"),
      ], now: now)
    #expect(groups.map(\.label) == ["오늘", "어제", "지난 7일", "지난 30일", "7월", "2025년 12월"])
    #expect(
      groups.map { $0.documents.map(\.item.entityId) } == [
        ["a"], ["b"], ["c"], ["d"], ["e"], ["f"],
      ])
  }

  @Test func mergesConsecutiveDocumentsOfTheSameBucket() {
    let groups = RecentGrouping.groups(
      [
        document("a", "2026-09-22T01:00:00.000Z"), document("b", "2026-09-21T20:00:00.000Z"),
        document("c", "2026-09-19T10:00:00.000Z"), document("d", "2026-09-18T10:00:00.000Z"),
      ], now: now)
    #expect(groups.map(\.id) == ["today", "week"])
    #expect(groups[0].documents.count == 2)
    #expect(groups[1].documents.count == 2)
  }

  @Test func skipsDocumentsWithoutViewedAt() {
    let groups = RecentGrouping.groups([document("a", nil)], now: now)
    #expect(groups.isEmpty)
  }
}
