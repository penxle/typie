import FactoryKit
import Observation

@MainActor @Observable
final class SearchSession {
  let model: SearchModel
  private(set) var focusRequest = 0
  private(set) var blurRequest = 0

  init(model: SearchModel) {
    self.model = model
  }

  convenience init() {
    self.init(model: Container.shared.searchModel())
  }

  func requestFocus() {
    focusRequest += 1
  }

  func releaseFocus() {
    blurRequest += 1
  }

  nonisolated static func searchPrompt(siteName: String?) -> String {
    guard let siteName else { return "검색" }
    return "\(truncated(siteName, to: 10))에서 검색..."
  }

  nonisolated private static func truncated(_ text: String, to length: Int) -> String {
    text.count > length ? "\(text.prefix(length))..." : text
  }
}
