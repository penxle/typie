import Foundation
import Testing

@testable import Core

@Suite struct KSTDayTests {
  @Test func convertsInstantsWithKoreanDayBoundary() {
    let secondsSinceEpoch = 1_800_000_000 - (1_800_000_000 % 86_400) + 15 * 3600
    let utcLateEvening = Date(timeIntervalSince1970: TimeInterval(secondsSinceEpoch))
    let kst = KSTDay(utcLateEvening)
    let utcSameDay = KSTDay(utcLateEvening.addingTimeInterval(-3600))
    #expect(kst == utcSameDay.adding(days: 1))
  }

  @Test func addsAndMeasuresDays() {
    let day = KSTDay(year: 2026, month: 2, day: 27)
    #expect(day.adding(days: 2) == KSTDay(year: 2026, month: 3, day: 1))
    #expect(day.days(to: day.adding(days: 10)) == 10)
    #expect(day.adding(days: -1).days(to: day) == 1)
  }

  @Test func weekdayStartsOnSunday() {
    #expect(KSTDay(year: 2026, month: 9, day: 20).weekday == 1)
    #expect(KSTDay(year: 2026, month: 9, day: 26).weekday == 7)
  }

  @Test func ordersByDate() {
    #expect(KSTDay(year: 2026, month: 1, day: 31) < KSTDay(year: 2026, month: 2, day: 1))
    #expect(KSTDay(year: 2025, month: 12, day: 31) < KSTDay(year: 2026, month: 1, day: 1))
  }

  @Test func usesKoreaStandardTimeRegardlessOfHost() throws {
    #expect(KST.calendar.timeZone.identifier == "Asia/Seoul")
    let formatter = ISO8601DateFormatter()
    let beforeBoundary = try #require(formatter.date(from: "2026-09-20T14:59:59Z"))
    let atBoundary = try #require(formatter.date(from: "2026-09-20T15:00:00Z"))
    #expect(KSTDay(beforeBoundary) == KSTDay(year: 2026, month: 9, day: 20))
    #expect(KSTDay(atBoundary) == KSTDay(year: 2026, month: 9, day: 21))
  }
}
