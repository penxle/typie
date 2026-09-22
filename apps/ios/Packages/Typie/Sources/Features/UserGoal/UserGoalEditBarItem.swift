#if canImport(UIKit)

  import Core
  import UIKit

  @MainActor
  enum UserGoalEditBarItem {
    static func bind(_ item: UINavigationItem, to model: UserGoalModel) {
      let button = item.rightBarButtonItem
      keepObserving(while: model) { [weak item, weak model] in
        guard let item, let model else { return }
        item.rightBarButtonItem = model.hasGoal ? button : nil
      }
    }
  }

#endif
