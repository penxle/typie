import Core
import Foundation

enum UserGoalFormat {
  static let weekdayNames = ["일", "월", "화", "수", "목", "금", "토"]

  static func characters(_ value: Int) -> String {
    "\(Formatting.comma(value))자"
  }

  static func dayLabel(_ day: KSTDay) -> String {
    "\(day.month)월 \(day.day)일 \(weekdayNames[day.weekday - 1])"
  }
}
