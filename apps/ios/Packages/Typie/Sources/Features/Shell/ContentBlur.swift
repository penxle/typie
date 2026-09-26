#if canImport(UIKit)

  import UIKit

  @MainActor
  enum ContentBlur {
    private static let filterName = "contentBlur"
    private static let keyPath = "filters.contentBlur.inputRadius"

    static func animate(_ views: [UIView], to radius: CGFloat, duration: TimeInterval) {
      for view in views {
        let layer = view.layer
        let from = (layer.presentation() ?? layer).value(forKeyPath: keyPath) as? CGFloat ?? 0
        if layer.filters?.isEmpty != false {
          guard let filter = makeFilter(type: "gaussianBlur") else { continue }
          filter.setValue(filterName, forKey: "name")
          filter.setValue(from, forKey: "inputRadius")
          layer.filters = [filter]
        }
        let animation = CABasicAnimation(keyPath: keyPath)
        animation.fromValue = from
        animation.toValue = radius
        animation.duration = duration
        animation.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        layer.setValue(radius, forKeyPath: keyPath)
        layer.add(animation, forKey: filterName)
      }
    }

    static func clear(_ views: [UIView]) {
      for view in views {
        view.layer.removeAnimation(forKey: filterName)
        view.layer.filters = nil
      }
    }

    static func makeFilter(type: String) -> NSObject? {
      guard let filterClass = NSClassFromString("CAFilter") as AnyObject as? NSObjectProtocol else {
        return nil
      }
      let selector = NSSelectorFromString("filterWithType:")
      guard filterClass.responds(to: selector),
        let filter = filterClass.perform(selector, with: type)?.takeUnretainedValue() as? NSObject
      else { return nil }
      return filter
    }
  }

#endif
