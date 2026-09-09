import Foundation
import ObjectiveC.runtime
import UIKit

// Session-scoped UIKit integration for the editor's Compose text input view.
@MainActor @objcMembers public final class EditorTextInputBridge: NSObject {
  private static weak var activeResponder: AnyObject?
  private static var installGeneration = 0
  private static var patchedClasses: Set<ObjectIdentifier> = []
  private static let documentNavigation = EditorDocumentNavigation()

  public static func install(on view: UIView) -> Int {
    installGeneration += 1
    let generation = installGeneration
    if let cls = object_getClass(view) {
      patchIfNeeded(on: cls)
    }
    activeResponder = view
    // Compose normally connects before focus. Native text views can already be focused.
    if view.isFirstResponder { view.reloadInputViews() }
    return generation
  }

  public static func uninstall(generation: Int) {
    guard generation == installGeneration else {
      return
    }
    installGeneration += 1
    activeResponder = nil
  }

  public static func takeDocumentNavigation(_ consume: (Bool, Bool) -> Void) {
    documentNavigation.takeMovement(consume)
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
    installDocumentNavigation(on: cls)
  }

  private static func installTraitGetter(cls: AnyClass, selector: Selector) {
    // Disable OS-level smart punctuation that would bypass the editor's undo state.
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

  private static func installDocumentNavigation(on cls: AnyClass) {
    // This selector is provided by our Compose edit-batching patch. Without it,
    // keep native behavior rather than ending composition before the host edit.
    let flushSelector = NSSelectorFromString("flushPendingTextInputEdits")
    guard let flushMethod = class_getInstanceMethod(cls, flushSelector),
      method_getNumberOfArguments(flushMethod) == 2
    else { return }
    let flushReturnType = method_copyReturnType(flushMethod)
    defer { free(flushReturnType) }
    guard String(cString: flushReturnType) == "v" else { return }
    typealias FlushMethod = @convention(c) (AnyObject, Selector) -> Void
    let flush = unsafeBitCast(method_getImplementation(flushMethod), to: FlushMethod.self)

    for name in ["_moveLeft:withHistory:", "_moveRight:withHistory:"] {
      let selector = NSSelectorFromString(name)
      guard let method = class_getInstanceMethod(cls, selector),
        method_getNumberOfArguments(method) == 4,
        let extendingType = method_copyArgumentType(method, 2),
        let historyType = method_copyArgumentType(method, 3)
      else { continue }
      let returnType = method_copyReturnType(method)
      defer {
        free(returnType)
        free(extendingType)
        free(historyType)
      }
      guard String(cString: returnType) == "@",
        ["B", "c"].contains(String(cString: extendingType)),
        String(cString: historyType) == "@"
      else { continue }

      typealias MoveMethod =
        @convention(c) (AnyObject, Selector, Bool, AnyObject?) -> Unmanaged<AnyObject>?
      let original = unsafeBitCast(method_getImplementation(method), to: MoveMethod.self)
      let block: @convention(block) (AnyObject, Bool, AnyObject?) -> AnyObject? = {
        object, extending, history in
        let move = { original(object, selector, extending, history)?.takeUnretainedValue() }
        guard let responder = object as? UIResponder, let input = object as? UITextInput else {
          return move()
        }
        let generation = installGeneration
        return documentNavigation.perform(
          on: input, backward: name == "_moveLeft:withHistory:", extending: extending,
          isCurrent: {
            generation == installGeneration && activeResponder === responder
              && responder.isFirstResponder
          },
          move: move,
          flush: { flush(object, flushSelector) },
          endComposition: { _ = EditorKeyboardBridge.endInputMethodComposition() }
        )
      }
      class_replaceMethod(
        cls, selector, imp_implementationWithBlock(block), method_getTypeEncoding(method))
    }
  }
}
