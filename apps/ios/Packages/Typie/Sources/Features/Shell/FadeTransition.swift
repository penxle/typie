#if canImport(UIKit)

  import UIKit

  final class FadeTransition: NSObject, UIViewControllerAnimatedTransitioning {
    private let duration: TimeInterval
    private var animator: UIViewPropertyAnimator?

    init(duration: TimeInterval) {
      self.duration = duration
    }

    func transitionDuration(using context: (any UIViewControllerContextTransitioning)?)
      -> TimeInterval
    {
      duration
    }

    func animateTransition(using context: any UIViewControllerContextTransitioning) {
      interruptibleAnimator(using: context).startAnimation()
    }

    func interruptibleAnimator(using context: any UIViewControllerContextTransitioning)
      -> any UIViewImplicitlyAnimating
    {
      if let animator { return animator }
      let animator = UIViewPropertyAnimator(duration: duration, curve: .easeOut)
      if let to = context.viewController(forKey: .to), let toView = context.view(forKey: .to) {
        toView.frame = context.finalFrame(for: to)
        toView.alpha = 0
        context.containerView.addSubview(toView)
        toView.layoutIfNeeded()
        animator.addAnimations { toView.alpha = 1 }
      }
      animator.addCompletion { _ in
        context.completeTransition(!context.transitionWasCancelled)
      }
      self.animator = animator
      return animator
    }

    func animationEnded(_ transitionCompleted: Bool) {
      animator = nil
    }
  }

#endif
