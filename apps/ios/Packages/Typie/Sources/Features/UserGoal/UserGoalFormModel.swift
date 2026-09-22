import Design
import Observation

@MainActor @Observable
final class UserGoalFormModel {
  nonisolated static let invalidMessage = "목표 글자 수를 올바르게 입력해 주세요."
  nonisolated static let rule = TFormRule { parse($0) == nil ? invalidMessage : nil }

  let form: TFormState
  let target: TFieldState
  let hasGoal: Bool
  private(set) var isSubmitting = false
  private(set) var isRemoving = false

  @ObservationIgnored private let goal: UserGoalModel

  init(goal: UserGoalModel) {
    self.goal = goal
    let form = TFormState(autoFocusFirstField: true, validatesOnBlur: false)
    let current = goal.state?.status.map { String($0.target) } ?? ""
    target = form.field(current, rules: [Self.rule])
    hasGoal = goal.hasGoal
    self.form = form
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
    guard !isSubmitting, !isRemoving else { return nil }
    guard form.validate(), let value = Self.parse(target.value) else { return nil }
    isSubmitting = true
    defer { isSubmitting = false }
    form.endEditing()
    return await goal.save(target: value)
  }

  func remove() async -> Bool {
    guard !isSubmitting, !isRemoving else { return false }
    isRemoving = true
    defer { isRemoving = false }
    form.endEditing()
    return await goal.remove()
  }
}
