import WebKit

typealias ClosureType = @convention(c) (Any, Selector, UnsafeRawPointer, Bool, Bool, Bool, Any?) -> Void

class AppWKWebView: WKWebView {
  @available(*, unavailable)
  required init?(coder: NSCoder) {
    fatalError("init(coder:) has not been implemented")
  }

  override init(frame: CGRect, configuration: WKWebViewConfiguration) {
    super.init(frame: frame, configuration: configuration)
  }

  override var inputAccessoryView: UIView? {
    return nil
  }

  override var inputAssistantItem: UITextInputAssistantItem {
    return UITextInputAssistantItem()
  }

  func setKeyboardRequiresUserInteraction(_ value: Bool) {
    guard let WKContentView: AnyClass = NSClassFromString("WKContentView") else {
      fatalError("Cannot find the WKContentView class")
    }

    let selector: Selector = sel_getUid("_elementDidFocus:userIsInteracting:blurPreviousNode:activityStateChanges:userObject:")

    if let method = class_getInstanceMethod(WKContentView, selector) {
      let imp: IMP = method_getImplementation(method)
      let closure: ClosureType = unsafeBitCast(imp, to: ClosureType.self)

      let block: @convention(block) (Any, UnsafeRawPointer, Bool, Bool, Bool, Any?) -> Void = { me, arg0, _, arg2, arg3, arg4 in
        closure(me, selector, arg0, !value, arg2, arg3, arg4)
      }

      let override: IMP = imp_implementationWithBlock(block)
      method_setImplementation(method, override)
    }
  }
}
