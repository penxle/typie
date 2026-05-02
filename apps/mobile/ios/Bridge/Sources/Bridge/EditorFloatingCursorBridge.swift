import Foundation
import ObjectiveC.runtime
import UIKit

@MainActor @objcMembers public final class EditorFloatingCursorBridge: NSObject {
  public static var onBegin: (() -> Void)?
  public static var onUpdate: ((Double, Double) -> Void)?
  public static var onEnd: (() -> Void)?

  private static var activeBeginPoint: CGPoint?
  private static weak var activeResponder: AnyObject?
  private static var installGeneration = 0
  private static var installedClasses: [ObjectIdentifier: InstalledClass] = [:]

  public static func install() -> Int {
    installGeneration += 1
    let generation = installGeneration
    installOnCurrentFirstResponder(generation: generation)

    DispatchQueue.main.async {
      installOnCurrentFirstResponder(generation: generation)
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.08) {
      installOnCurrentFirstResponder(generation: generation)
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.24) {
      installOnCurrentFirstResponder(generation: generation)
    }

    return generation
  }

  public static func clearHandlersForInstall(generation: Int) {
    guard generation == installGeneration else {
      return
    }
    installGeneration += 1
    clearActiveHandlers()
  }

  private static func clearActiveHandlers() {
    onBegin = nil
    onUpdate = nil
    onEnd = nil
    activeBeginPoint = nil
    activeResponder = nil
  }

  private static func installOnCurrentFirstResponder(generation: Int) {
    guard generation == installGeneration else {
      return
    }

    guard
      let responder = UIApplication.shared.activeWindow?.typieFirstResponder(),
      let cls: AnyClass = object_getClass(responder)
    else {
      return
    }

    if installIfNeeded(on: cls) {
      activeResponder = responder
    }
  }

  private static func installIfNeeded(on cls: AnyClass) -> Bool {
    let key = ObjectIdentifier(cls)
    if installedClasses[key] != nil {
      return true
    }

    guard
      let beginMethod = class_getInstanceMethod(cls, Selectors.begin),
      let updateMethod = class_getInstanceMethod(cls, Selectors.update),
      let endMethod = class_getInstanceMethod(cls, Selectors.end)
    else {
      return false
    }

    let installed = InstalledClass(
      begin: method_getImplementation(beginMethod),
      update: method_getImplementation(updateMethod),
      end: method_getImplementation(endMethod)
    )
    installedClasses[key] = installed

    replacePointMethod(
      cls: cls,
      selector: Selectors.begin,
      method: beginMethod,
      original: installed.begin
    ) { object, selector, point, original in
      if shouldHandle(object) {
        activeBeginPoint = point
        onBegin?()
      } else {
        original(object, selector, point)
      }
    }

    replacePointMethod(
      cls: cls,
      selector: Selectors.update,
      method: updateMethod,
      original: installed.update
    ) { object, selector, point, original in
      if shouldHandle(object) {
        let begin = activeBeginPoint ?? point
        onUpdate?(Double(point.x - begin.x), Double(point.y - begin.y))
      } else {
        original(object, selector, point)
      }
    }

    replaceVoidMethod(
      cls: cls,
      selector: Selectors.end,
      method: endMethod,
      original: installed.end
    ) { object, selector, original in
      if shouldHandle(object) {
        activeBeginPoint = nil
        onEnd?()
      } else {
        original(object, selector)
      }
    }

    return true
  }

  private static func shouldHandle(_ object: AnyObject) -> Bool {
    (onBegin != nil || onUpdate != nil || onEnd != nil) && activeResponder === object
  }

  private static func replacePointMethod(
    cls: AnyClass,
    selector: Selector,
    method: Method,
    original: IMP,
    handler: @escaping (
      _ object: AnyObject,
      _ selector: Selector,
      _ point: CGPoint,
      _ original: @escaping PointMethod
    ) -> Void
  ) {
    let originalMethod = unsafeBitCast(original, to: PointMethod.self)
    let block: @convention(block) (AnyObject, CGPoint) -> Void = {
      object,
      point in
      handler(object, selector, point, originalMethod)
    }
    class_replaceMethod(
      cls,
      selector,
      imp_implementationWithBlock(block),
      method_getTypeEncoding(method)
    )
  }

  private static func replaceVoidMethod(
    cls: AnyClass,
    selector: Selector,
    method: Method,
    original: IMP,
    handler: @escaping (
      _ object: AnyObject,
      _ selector: Selector,
      _ original: @escaping VoidMethod
    ) -> Void
  ) {
    let originalMethod = unsafeBitCast(original, to: VoidMethod.self)
    let block: @convention(block) (AnyObject) -> Void = { object in
      handler(object, selector, originalMethod)
    }
    class_replaceMethod(
      cls,
      selector,
      imp_implementationWithBlock(block),
      method_getTypeEncoding(method)
    )
  }

  private enum Selectors {
    static let begin = NSSelectorFromString("beginFloatingCursorAtPoint:")
    static let update = NSSelectorFromString("updateFloatingCursorAtPoint:")
    static let end = NSSelectorFromString("endFloatingCursor")
  }

  private struct InstalledClass {
    let begin: IMP
    let update: IMP
    let end: IMP
  }

  private typealias PointMethod = @convention(c) (
    AnyObject,
    Selector,
    CGPoint
  ) -> Void
  private typealias VoidMethod = @convention(c) (AnyObject, Selector) -> Void
}

private extension UIView {
  @MainActor
  func typieFirstResponder() -> UIResponder? {
    if isFirstResponder {
      return self
    }

    for subview in subviews {
      if let responder = subview.typieFirstResponder() {
        return responder
      }
    }

    return nil
  }
}
