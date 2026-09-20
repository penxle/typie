import Foundation

public enum KST {
  public static let timeZone = TimeZone(identifier: "Asia/Seoul")!

  public static let calendar: Calendar = {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = timeZone
    calendar.firstWeekday = 1
    return calendar
  }()
}
