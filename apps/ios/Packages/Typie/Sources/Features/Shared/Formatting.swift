import Foundation

enum Formatting {
  static func comma(_ value: Int) -> String {
    value.formatted(.number.grouping(.automatic).locale(Locale(identifier: "en_US")))
  }
}
