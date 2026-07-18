import UIKit

/// Workaround for https://youtrack.jetbrains.com/issue/CMP-9895.
///
/// Compose Multiplatform 1.11.1 does not translate iOS indirect pinch input from `UIEvent`
/// into its common scale pointer events. This bridge recognizes only UIKit zero-touch pinches
/// and forwards them to the editor's shared zoom semantic. Remove it when Compose provides that
/// event path.
@MainActor @objcMembers public final class EditorIndirectScaleGestureBridge: NSObject,
  UIGestureRecognizerDelegate
{
  public var onShouldBegin: ((Double, Double) -> Bool)?
  public var onBegin: (() -> Bool)?
  public var onScale: ((Double, Double, Double) -> Bool)?
  public var onEnd: (() -> Void)?

  private weak var view: UIView?
  private var recognizer: UIPinchGestureRecognizer!
  private var active = false

  public init(view: UIView) {
    self.view = view
    super.init()

    recognizer = UIPinchGestureRecognizer(target: self, action: #selector(handlePinch(_:)))
    recognizer.cancelsTouchesInView = false
    recognizer.delaysTouchesBegan = false
    recognizer.delaysTouchesEnded = false
    recognizer.delegate = self
    view.addGestureRecognizer(recognizer)
  }

  public func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
    guard
      gestureRecognizer === recognizer,
      recognizer.numberOfTouches == 0,
      let view
    else {
      return false
    }
    let focal = recognizer.location(in: view)
    return onShouldBegin?(focal.x, focal.y) ?? false
  }

  public func gestureRecognizer(
    _ gestureRecognizer: UIGestureRecognizer,
    shouldRecognizeSimultaneouslyWith otherGestureRecognizer: UIGestureRecognizer
  ) -> Bool {
    true
  }

  public func endActive() {
    guard active else {
      return
    }
    active = false
    onEnd?()
  }

  public func dispose() {
    endActive()
    recognizer.delegate = nil
    view?.removeGestureRecognizer(recognizer)
    onShouldBegin = nil
    onBegin = nil
    onScale = nil
    onEnd = nil
  }

  @objc private func handlePinch(_ recognizer: UIPinchGestureRecognizer) {
    guard let view else {
      endActive()
      return
    }

    switch recognizer.state {
    case .began:
      active = recognizer.numberOfTouches == 0 && (onBegin?() ?? false)
      recognizer.scale = 1
    case .changed:
      guard active else {
        return
      }
      let focal = recognizer.location(in: view)
      let scaleFactor = recognizer.scale
      recognizer.scale = 1
      if !(onScale?(focal.x, focal.y, scaleFactor) ?? false) {
        endActive()
      }
    case .ended, .cancelled, .failed:
      endActive()
    default:
      break
    }
  }
}
