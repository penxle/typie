import Foundation

public func timeAgo(_ date: Date, now: Date = Date()) -> String {
  let elapsed = now.timeIntervalSince(date)
  let isPast = elapsed > 0
  let seconds = abs(elapsed)
  let minute = 60.0
  let hour = 60 * minute
  let day = 24 * hour
  let text: String
  switch seconds {
  case ..<minute: return "방금"
  case ..<hour: text = "\(Int(seconds / minute))분"
  case ..<day: text = "\(Int(seconds / hour))시간"
  case ..<(30 * day): text = "\(Int(seconds / day))일"
  case ..<(365 * day): text = "\(Int(seconds / day) / 30)개월"
  default: text = "\(Int(seconds / day) / 365)년"
  }
  return isPast ? "\(text) 전" : "\(text) 후"
}

public func parseDateTime(_ raw: String) -> Date? {
  let fractional = ISO8601DateFormatter()
  fractional.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
  if let date = fractional.date(from: raw) { return date }
  let plain = ISO8601DateFormatter()
  plain.formatOptions = [.withInternetDateTime]
  return plain.date(from: raw)
}
