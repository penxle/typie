import Foundation
import ObjectiveC.runtime
import QuartzCore
import UIKit

struct SoftwareKeyboardPresentationGeometry {
  let shownHostBounds: CGRect
  let shownFrame: CGRect
  let hostTravel: CGFloat
  let frameTravel: CGFloat

  func hostBounds(at progress: CGFloat) -> CGRect {
    shownHostBounds.offsetBy(dx: 0, dy: -hostTravel * progress.clamped01)
  }

  func keyboardFrame(at progress: CGFloat) -> CGRect {
    shownFrame.offsetBy(dx: 0, dy: frameTravel * progress.clamped01)
  }

  func progress(for hostBounds: CGRect) -> CGFloat {
    guard hostTravel > 0 else { return 0 }
    return ((shownHostBounds.minY - hostBounds.minY) / hostTravel).clamped01
  }
}

/// Drives a docked UIKit software keyboard from an app-owned interactive transition.
///
/// The private keyboard host bounds provide the visual position. Matching synthetic keyboard
/// frame notifications keep Compose `WindowInsets.ime` and Typie keyboard geometry in the same frame.
@MainActor @objcMembers public final class SoftwareKeyboardPresentationBridge: NSObject {
  public var onInvalidated: (() -> Void)?
  public var onAccepted: (() -> Void)?

  public var hiddenProgress: Double {
    get { Double(progress) }
    set { updateHiddenProgress(CGFloat(newValue)) }
  }

  private enum Endpoint: Equatable {
    case shown
    case hidden

    var progress: CGFloat { self == .hidden ? 1 : 0 }
  }

  private enum Lifecycle {
    case idle
    case interactive
    case animating(Endpoint)
    case hiddenAccepted
    case closed
  }

  private var lifecycle: Lifecycle = .idle
  private weak var appWindow: UIWindow?
  private weak var hostView: UIView?
  private var geometry: SoftwareKeyboardPresentationGeometry?
  private var progress: CGFloat = 0

  private var animator: UIViewPropertyAnimator?
  private var displayLink: CADisplayLink?
  private var hideRetryTask: Task<Void, Never>?
  private var selfRetainer: SoftwareKeyboardPresentationBridge?
  private var observing = false

  public override init() {
    super.init()
  }

  public func acquire() -> Bool {
    guard case .idle = lifecycle else { return false }
    guard
      let window = Self.currentKeyWindow(),
      let keyboard = Self.currentDockedKeyboard(in: window)
    else {
      lifecycle = .closed
      return false
    }

    appWindow = window
    hostView = keyboard.hostView
    geometry = keyboard.geometry
    progress = 0
    lifecycle = .interactive
    startObserving()
    return true
  }

  public func finishShown() {
    finish(.shown)
  }

  public func finishHidden() {
    finish(.hidden)
  }

  public func dispose() {
    switch lifecycle {
    case .interactive, .animating:
      lifecycle = .closed
      stopEndpointAnimation()
      stopObserving()
      setHostProgress(0)
      postFrame(progress: 0, settled: true)
      clearCallbacksAndReferences()

    case .hiddenAccepted:
      // Keep the keyboard offscreen until UIKit completes its semantic dismissal.
      onInvalidated = nil
      onAccepted = nil
      selfRetainer = self

    case .idle, .closed:
      lifecycle = .closed
      stopEndpointAnimation()
      stopObserving()
      clearCallbacksAndReferences()
    }
  }

  private func updateHiddenProgress(_ value: CGFloat) {
    guard case .interactive = lifecycle else { return }
    let next = value.clamped01
    guard hostView?.window != nil else {
      invalidate()
      return
    }
    setHostProgress(next)
    postFrame(progress: next)
  }

  private func finish(_ endpoint: Endpoint) {
    guard case .interactive = lifecycle else { return }
    guard let hostView, hostView.window != nil else {
      invalidate()
      return
    }
    let target = endpoint.progress
    lifecycle = .animating(endpoint)
    if abs(progress - target) < 0.000_1 {
      setHostProgress(target)
      postFrame(progress: target, settled: true)
      complete(endpoint)
      return
    }

    let timing =
      UICubicTimingParameters(
        controlPoint1: CGPoint(x: 0.4, y: 0),
        controlPoint2: CGPoint(x: 0.2, y: 1)
      )
    let animator = UIViewPropertyAnimator(duration: 0.22, timingParameters: timing)
    self.animator = animator
    animator.addAnimations { [weak self, weak hostView] in
      guard let self, let hostView, let geometry = self.geometry else { return }
      hostView.bounds = geometry.hostBounds(at: target)
    }
    animator.addCompletion { [weak self] position in
      guard
        let self,
        case .animating(let activeEndpoint) = self.lifecycle,
        activeEndpoint == endpoint
      else {
        return
      }
      if position == .end {
        guard self.hostView?.window != nil else {
          self.invalidate()
          return
        }
        self.setHostProgress(target)
        self.postFrame(progress: target, settled: true)
        self.complete(endpoint)
      } else if case .animating = self.lifecycle {
        self.invalidate(restoringShownFrame: true)
      }
    }
    startDisplayLink()
    animator.startAnimation()
  }

  @objc private func displayLinkAdvanced(_ link: CADisplayLink) {
    switch lifecycle {
    case .animating:
      guard let hostView, hostView.window != nil else {
        invalidate()
        return
      }
      guard let presentation = hostView.layer.presentation() else { return }
      guard let geometry else {
        invalidate()
        return
      }
      let current = geometry.progress(for: presentation.bounds)
      postFrame(progress: current)

    case .hiddenAccepted:
      guard let hostView, let window = hostView.window, !window.isHidden, !hostView.isHidden else {
        completeHiddenCleanup()
        return
      }
      setHostProgress(1)

    case .idle, .interactive, .closed:
      stopDisplayLink()
    }
  }

  private func complete(_ endpoint: Endpoint) {
    animator = nil
    switch endpoint {
    case .shown:
      lifecycle = .closed
      stopDisplayLink()
      stopObserving()
      let accepted = onAccepted
      onAccepted = nil
      onInvalidated = nil
      clearReferences()
      accepted?()

    case .hidden:
      lifecycle = .hiddenAccepted
      selfRetainer = self
      startDisplayLink()
      startHideRetryTask()
      let accepted = onAccepted
      onAccepted = nil
      onInvalidated = nil
      requestNativeHide()
      accepted?()
    }
  }

  private func setHostProgress(_ value: CGFloat) {
    guard let hostView, let geometry else { return }
    CATransaction.begin()
    CATransaction.setDisableActions(true)
    hostView.layer.bounds = geometry.hostBounds(at: value)
    CATransaction.commit()
  }

  private func postFrame(progress: CGFloat, settled: Bool = false) {
    guard let geometry else { return }
    let next = progress.clamped01
    let begin = geometry.keyboardFrame(at: self.progress)
    let end = geometry.keyboardFrame(at: next)
    self.progress = next
    let userInfo: [AnyHashable: Any] = [
      UIResponder.keyboardFrameBeginUserInfoKey: NSValue(cgRect: begin),
      UIResponder.keyboardFrameEndUserInfoKey: NSValue(cgRect: end),
      UIResponder.keyboardAnimationDurationUserInfoKey: 0.0,
      UIResponder.keyboardAnimationCurveUserInfoKey: UIView.AnimationCurve.linear.rawValue,
      UIResponder.keyboardIsLocalUserInfoKey: true,
    ]
    let center = NotificationCenter.default
    center.post(
      name: UIResponder.keyboardWillChangeFrameNotification,
      object: self,
      userInfo: userInfo
    )
    if settled {
      center.post(
        name: UIResponder.keyboardDidChangeFrameNotification,
        object: self,
        userInfo: userInfo
      )
    }
  }

  private func requestNativeHide() {
    // Preserve first responder ownership until the common controller accepts the visual endpoint.
    if Self.invokeKeyboardImpl("dismissKeyboard") { return }
    forceResignFirstResponder()
  }

  private func forceResignFirstResponder() {
    if appWindow?.endEditing(true) == true { return }
    UIApplication.shared.sendAction(
      #selector(UIResponder.resignFirstResponder),
      to: nil,
      from: nil,
      for: nil
    )
  }

  private func startHideRetryTask() {
    hideRetryTask?.cancel()
    hideRetryTask = Task { @MainActor [weak self] in
      var attempts = 0
      while let self {
        do {
          try await Task.sleep(nanoseconds: 300_000_000)
        } catch {
          return
        }
        guard case .hiddenAccepted = self.lifecycle else { return }
        guard let hostView = self.hostView, let window = hostView.window, !window.isHidden else {
          self.completeHiddenCleanup()
          return
        }
        self.setHostProgress(1)
        attempts += 1
        if attempts < 2 {
          self.requestNativeHide()
        } else {
          self.forceResignFirstResponder()
        }
      }
    }
  }

  private func startDisplayLink() {
    guard displayLink == nil else { return }
    let link = CADisplayLink(target: self, selector: #selector(displayLinkAdvanced(_:)))
    displayLink = link
    link.add(to: .main, forMode: .common)
  }

  private func stopDisplayLink() {
    displayLink?.invalidate()
    displayLink = nil
  }

  private func stopEndpointAnimation() {
    animator?.stopAnimation(true)
    animator = nil
    stopDisplayLink()
  }

  private func startObserving() {
    guard !observing else { return }
    observing = true
    let center = NotificationCenter.default
    for name in [
      UIResponder.keyboardWillShowNotification,
      UIResponder.keyboardWillHideNotification,
      UIResponder.keyboardWillChangeFrameNotification,
      UIResponder.keyboardDidShowNotification,
      UIResponder.keyboardDidChangeFrameNotification,
    ] {
      center.addObserver(
        self,
        selector: #selector(keyboardMutated(_:)),
        name: name,
        object: nil
      )
    }
    center.addObserver(
      self,
      selector: #selector(keyboardDidHide(_:)),
      name: UIResponder.keyboardDidHideNotification,
      object: nil
    )
    center.addObserver(
      self,
      selector: #selector(applicationDidEnterBackground(_:)),
      name: UIApplication.didEnterBackgroundNotification,
      object: nil
    )
  }

  private func stopObserving() {
    guard observing else { return }
    observing = false
    NotificationCenter.default.removeObserver(self)
  }

  @objc private func keyboardMutated(_ notification: Notification) {
    handleNativeMutation(notification)
  }

  @objc private func keyboardDidHide(_ notification: Notification) {
    guard !isSynthetic(notification) else { return }
    switch lifecycle {
    case .hiddenAccepted:
      completeHiddenCleanup()
    case .interactive, .animating:
      invalidate()
    case .idle, .closed:
      break
    }
  }

  @objc private func applicationDidEnterBackground(_ notification: Notification) {
    switch lifecycle {
    case .interactive, .animating:
      invalidate(restoringShownFrame: true)
    case .hiddenAccepted:
      completeHiddenCleanup()
    case .idle, .closed:
      break
    }
  }

  private func handleNativeMutation(_ notification: Notification) {
    guard !isSynthetic(notification) else { return }
    switch lifecycle {
    case .interactive, .animating:
      invalidate()

    case .hiddenAccepted:
      setHostProgress(1)
      Task { @MainActor [weak self] in
        guard let self, case .hiddenAccepted = self.lifecycle else { return }
        self.postFrame(progress: 1, settled: true)
        self.requestNativeHide()
      }

    case .idle, .closed:
      break
    }
  }

  private func isSynthetic(_ notification: Notification) -> Bool {
    guard let object = notification.object else { return false }
    return (object as AnyObject) === self
  }

  private func invalidate(restoringShownFrame: Bool = false) {
    switch lifecycle {
    case .interactive, .animating:
      lifecycle = .closed
      stopEndpointAnimation()
      stopObserving()
      setHostProgress(0)
      if restoringShownFrame {
        postFrame(progress: 0, settled: true)
      }
      let invalidated = onInvalidated
      onAccepted = nil
      onInvalidated = nil
      clearReferences()
      invalidated?()
    case .idle, .hiddenAccepted, .closed:
      break
    }
  }

  private func completeHiddenCleanup() {
    guard case .hiddenAccepted = lifecycle else { return }
    lifecycle = .closed
    stopEndpointAnimation()
    hideRetryTask?.cancel()
    hideRetryTask = nil
    stopObserving()
    setHostProgress(0)
    clearCallbacksAndReferences()
    selfRetainer = nil
  }

  private func clearCallbacksAndReferences() {
    onAccepted = nil
    onInvalidated = nil
    clearReferences()
  }

  private func clearReferences() {
    appWindow = nil
    hostView = nil
    geometry = nil
    progress = 0
  }

  private struct DockedKeyboard {
    let hostView: UIView
    let geometry: SoftwareKeyboardPresentationGeometry
  }

  private static func currentDockedKeyboard(in appWindow: UIWindow) -> DockedKeyboard? {
    // Floating and split keyboards are deliberately left to UIKit.
    appWindow.layoutIfNeeded()
    let screen = appWindow.screen
    let screenBounds = screen.bounds
    guard
      let hostView = remoteKeyboardHost(on: screen),
      let hostFrame = frameInScreen(hostView),
      isDocked(hostFrame, in: screenBounds)
    else {
      return nil
    }

    let layoutFrame = appWindow.keyboardLayoutGuide.layoutFrame
    let frameFromGuide = screen.coordinateSpace.convert(layoutFrame, from: appWindow)
    let shownFrame: CGRect
    if isDocked(frameFromGuide, in: screenBounds) {
      shownFrame = frameFromGuide
    } else {
      shownFrame = hostFrame
    }

    guard isDocked(shownFrame, in: screenBounds) else { return nil }
    let hostTravel = screenBounds.maxY - hostFrame.minY
    let frameTravel = screenBounds.maxY - shownFrame.minY
    guard hostTravel > 0, frameTravel > 0 else { return nil }
    return DockedKeyboard(
      hostView: hostView,
      geometry: SoftwareKeyboardPresentationGeometry(
        shownHostBounds: hostView.layer.bounds,
        shownFrame: shownFrame,
        hostTravel: hostTravel,
        frameTravel: frameTravel
      )
    )
  }

  private static func remoteKeyboardHost(on screen: UIScreen) -> UIView? {
    guard let window = remoteKeyboardWindow(on: screen) else { return nil }
    window.layoutIfNeeded()
    return descendants(of: window).first { view in
      NSStringFromClass(type(of: view)).contains("UIInputSetHostView")
        && isVisible(view)
        && view.transform == .identity
        && (view.layer.animationKeys()?.isEmpty ?? true)
    }
  }

  private static func remoteKeyboardWindow(on screen: UIScreen) -> UIWindow? {
    guard let cls = NSClassFromString("UIRemoteKeyboardWindow") else { return nil }
    let selector = NSSelectorFromString("remoteKeyboardWindowForScreen:create:")
    guard
      let method = class_getClassMethod(cls, selector),
      method_getNumberOfArguments(method) == 4
    else {
      return nil
    }
    typealias Implementation = @convention(c) (AnyClass, Selector, UIScreen, Bool) -> UIWindow?
    let implementation = unsafeBitCast(method_getImplementation(method), to: Implementation.self)
    return implementation(cls, selector, screen, false)
  }

  private static func isDocked(_ frame: CGRect, in screen: CGRect) -> Bool {
    !frame.isNull
      && !frame.isInfinite
      && !frame.isEmpty
      && frame.height > minimumKeyboardHeight
      && frame.width >= screen.width * 0.7
      && frame.minY < screen.maxY - minimumKeyboardHeight
      && frame.maxY >= screen.maxY - 12
      && frame.intersects(screen)
  }

  private static func frameInScreen(_ view: UIView) -> CGRect? {
    guard let window = view.window else { return nil }
    let frameInWindow = view.convert(view.bounds, to: window)
    return window.screen.coordinateSpace.convert(frameInWindow, from: window)
  }

  private static func descendants(of root: UIView) -> [UIView] {
    var result: [UIView] = []
    var pending = root.subviews
    while let view = pending.popLast() {
      result.append(view)
      pending.append(contentsOf: view.subviews)
    }
    return result
  }

  private static func isVisible(_ view: UIView) -> Bool {
    guard let window = view.window, !window.isHidden, window.alpha > 0.01 else { return false }
    var current: UIView? = view
    while let view = current {
      if view.isHidden || view.alpha <= 0.01 { return false }
      current = view.superview
    }
    return true
  }

  private static func allWindows() -> [UIWindow] {
    UIApplication.shared.connectedScenes
      .compactMap { $0 as? UIWindowScene }
      .flatMap(\.windows)
  }

  private static func currentKeyWindow() -> UIWindow? {
    allWindows().first(where: \.isKeyWindow)
  }

  private static func invokeKeyboardImpl(_ selectorName: String) -> Bool {
    guard let cls = NSClassFromString("UIKeyboardImpl") as? NSObject.Type else { return false }
    var keyboard: NSObject?
    for instanceSelector in ["activeInstance", "sharedInstance"] {
      let selector = NSSelectorFromString(instanceSelector)
      if cls.responds(to: selector) {
        keyboard = cls.perform(selector)?.takeUnretainedValue() as? NSObject
      }
      if keyboard != nil { break }
    }
    guard let keyboard else { return false }
    let selector = NSSelectorFromString(selectorName)
    guard keyboard.responds(to: selector) else { return false }
    _ = keyboard.perform(selector)
    return true
  }

  private static let minimumKeyboardHeight: CGFloat = 44
}

extension CGFloat {
  fileprivate var clamped01: CGFloat { Swift.min(1, Swift.max(0, self)) }
}
