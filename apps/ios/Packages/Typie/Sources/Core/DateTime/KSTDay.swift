import Foundation

public struct KSTDay: Hashable, Sendable, Comparable {
  public let year: Int
  public let month: Int
  public let day: Int

  public init(year: Int, month: Int, day: Int) {
    self.year = year
    self.month = month
    self.day = day
  }

  public init(_ date: Date) {
    let components = KST.calendar.dateComponents([.year, .month, .day], from: date)
    self.init(year: components.year!, month: components.month!, day: components.day!)
  }

  public var date: Date {
    KST.calendar.date(from: DateComponents(year: year, month: month, day: day))!
  }

  public var weekday: Int {
    KST.calendar.component(.weekday, from: date)
  }

  public func adding(days: Int) -> KSTDay {
    KSTDay(KST.calendar.date(byAdding: .day, value: days, to: date)!)
  }

  public func days(to other: KSTDay) -> Int {
    KST.calendar.dateComponents([.day], from: date, to: other.date).day!
  }

  public static func < (lhs: KSTDay, rhs: KSTDay) -> Bool {
    (lhs.year, lhs.month, lhs.day) < (rhs.year, rhs.month, rhs.day)
  }
}
