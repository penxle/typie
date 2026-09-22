import Observation

@MainActor @Observable
final class UserGoalFormModel {
  nonisolated static let step = 100
  nonisolated static let largeStep = 1000
  nonisolated static let stepFloor = 100
  nonisolated static let minimum = 1
  nonisolated static let maximum = Int(Int32.max)
  nonisolated static let defaultTarget = 1000

  let hasGoal: Bool
  let initial: Int?
  private(set) var value: Int
  private(set) var isSubmitting = false
  private(set) var isRemoving = false

  @ObservationIgnored private let goal: UserGoalModel

  init(goal: UserGoalModel) {
    self.goal = goal
    initial = goal.state?.status?.target
    value = initial ?? Self.defaultTarget
    hasGoal = goal.hasGoal
  }

  var canSubmit: Bool { value >= Self.minimum && value <= Self.maximum && value != initial }

  var canDecrement: Bool { value > Self.stepFloor }
  var canIncrement: Bool { value < Self.maximum }

  func adjust(by delta: Int) {
    let floor = delta < 0 ? Self.stepFloor : Self.minimum
    value = min(Self.maximum, max(floor, value + delta))
  }

  @discardableResult
  func enter(_ raw: String) -> Bool {
    guard let parsed = Self.parse(raw) else { return false }
    value = parsed
    return true
  }

  nonisolated static func parse(_ raw: String) -> Int? {
    let trimmed = raw.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: ",", with: "")
    guard !trimmed.isEmpty, trimmed.allSatisfy(\.isNumber), let value = Int32(trimmed), value > 0
    else {
      return nil
    }
    return Int(value)
  }

  func submit() async -> Bool? {
    guard !isSubmitting, !isRemoving, canSubmit else { return nil }
    isSubmitting = true
    defer { isSubmitting = false }
    return await goal.save(target: value)
  }

  func remove() async -> Bool {
    guard !isSubmitting, !isRemoving else { return false }
    isRemoving = true
    defer { isRemoving = false }
    return await goal.remove()
  }
}
