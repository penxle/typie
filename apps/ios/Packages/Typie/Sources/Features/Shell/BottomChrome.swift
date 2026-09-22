import Foundation
import Observation

@MainActor @Observable
public final class BottomChrome {
  public private(set) var inset: CGFloat = 0

  public init() {}

  func set(inset: CGFloat) {
    guard self.inset != inset else { return }
    self.inset = inset
  }
}
