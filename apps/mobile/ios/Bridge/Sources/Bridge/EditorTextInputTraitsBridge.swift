import Foundation
import ObjectiveC.runtime
import UIKit

// Compose's text input view does not implement the UITextInputTraits smart
// punctuation getters, so UIKit applies OS-level smart dashes/quotes/deletes
// upstream of the editor core, which then cannot attribute or undo them. While
// an editor session is active, answer .no for the session's responder; every
// other responder keeps .default, matching an unimplemented getter.
@MainActor @objcMembers public final class EditorTextInputTraitsBridge: NSObject {
  private static weak var activeResponder: AnyObject?
  private static var installGeneration = 0
  private static var patchedClasses: Set<ObjectIdentifier> = []

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

  public static func uninstall(generation: Int) {
    guard generation == installGeneration else {
      return
    }
    installGeneration += 1
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

    patchIfNeeded(on: cls)
    if activeResponder !== responder {
      activeResponder = responder
      responder.reloadInputViews()
    }
  }

  private static func patchIfNeeded(on cls: AnyClass) {
    let key = ObjectIdentifier(cls)
    if patchedClasses.contains(key) {
      return
    }
    patchedClasses.insert(key)

    for name in ["smartDashesType", "smartQuotesType", "smartInsertDeleteType"] {
      installTraitGetter(cls: cls, selector: NSSelectorFromString(name))
    }
  }

  private static func installTraitGetter(cls: AnyClass, selector: Selector) {
    // UITextSmart*Type raw values: 0 = default, 1 = no.
    let block: @convention(block) (AnyObject) -> Int = { object in
      object === activeResponder ? 1 : 0
    }
    let imp = imp_implementationWithBlock(block)
    if let method = class_getInstanceMethod(cls, selector) {
      class_replaceMethod(cls, selector, imp, method_getTypeEncoding(method))
    } else {
      class_addMethod(cls, selector, imp, "q@:")
    }
  }
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
