#if canImport(UIKit)

  import Design
  import UIKit

  @available(iOS 26, *)
  final class CreateButtonView: UIView {
    static let side: CGFloat = MainTabBar.barHeight
    static let spacing: CGFloat = 12
    static let cardWidth: CGFloat = 208
    private static let fade: TimeInterval = 0.15
    private static let expandDuration: TimeInterval = 0.35
    private static let expandBounce: CGFloat = 0.3
    private static let collapseDuration: TimeInterval = 0.3
    private static let collapseBounce: CGFloat = 0.15
    private static let glyphExitDuration: TimeInterval = 0.12
    private static let glyphEnterDuration: TimeInterval = 0.2
    private static let glyphEnterDelay: TimeInterval = 0.1
    private static let glyphSlideDistance: CGFloat = 24

    let button = UIButton(type: .system)
    var onExpandedChange: ((Bool) -> Void)?
    var onHighlightChange: ((Int?) -> Void)?
    var onCommit: ((Int) -> Void)?
    private let glass = UIVisualEffectView(effect: nil)
    private let menuView: UIView
    private let state: CreateMenuState
    private let tint: UIColor
    private let glassWidth: NSLayoutConstraint
    private let glassHeight: NSLayoutConstraint
    private var glyphAnimator: UIViewPropertyAnimator?
    private var didConfigureCorners = false
    private(set) var isShown = false
    private(set) var isExpanded = false

    init(tint: UIColor, glyphTint: UIColor, state: CreateMenuState, menuView: UIView) {
      self.tint = tint
      self.state = state
      self.menuView = menuView
      glassWidth = glass.widthAnchor.constraint(equalToConstant: Self.side)
      glassHeight = glass.heightAnchor.constraint(equalToConstant: Self.side)
      super.init(frame: .zero)
      button.configuration = .plain()
      button.configuration?.baseForegroundColor = glyphTint
      button.configuration?.image = MainTabBar.barIcon(LucideIcon.plus)
      button.accessibilityLabel = "새로 만들기"
      button.addAction(
        UIAction { [weak self] _ in self?.setExpanded(true) }, for: .primaryActionTriggered)
      button.alpha = 0
      menuView.backgroundColor = .clear
      menuView.isUserInteractionEnabled = false
      isMultipleTouchEnabled = false
      addSubview(glass)
      glass.contentView.addSubview(menuView)
      glass.contentView.addSubview(button)
      glass.translatesAutoresizingMaskIntoConstraints = false
      button.translatesAutoresizingMaskIntoConstraints = false
      NSLayoutConstraint.activate([
        glassWidth, glassHeight,
        glass.trailingAnchor.constraint(equalTo: trailingAnchor),
        glass.bottomAnchor.constraint(equalTo: bottomAnchor),
        button.widthAnchor.constraint(equalToConstant: Self.side),
        button.heightAnchor.constraint(equalToConstant: Self.side),
        button.trailingAnchor.constraint(equalTo: glass.contentView.trailingAnchor),
        button.bottomAnchor.constraint(equalTo: glass.contentView.bottomAnchor),
      ])
      isHidden = true
      isUserInteractionEnabled = false
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    var cardHeight: CGFloat {
      CreateMenu.padding * 2 + CreateMenu.rowHeight * CGFloat(state.items.count)
    }

    override var intrinsicContentSize: CGSize {
      CGSize(width: Self.cardWidth, height: max(Self.side, cardHeight))
    }

    override func layoutSubviews() {
      super.layoutSubviews()
      if !didConfigureCorners {
        didConfigureCorners = true
        glass.cornerConfiguration = .capsule(maximumRadius: Self.side / 2)
      }
      menuView.frame = CGRect(
        x: CreateMenu.padding, y: CreateMenu.padding,
        width: Self.cardWidth - CreateMenu.padding * 2, height: cardHeight - CreateMenu.padding * 2)
    }

    override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
      if isExpanded { return bounds.contains(point) ? self : nil }
      return glass.frame.contains(point) ? super.hitTest(point, with: event) : nil
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
      guard isExpanded, let touch = touches.first else {
        return super.touchesBegan(touches, with: event)
      }
      scrub(at: touch.location(in: self))
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
      guard isExpanded, let touch = touches.first else {
        return super.touchesMoved(touches, with: event)
      }
      scrub(at: touch.location(in: self))
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
      guard isExpanded, let touch = touches.first else {
        return super.touchesEnded(touches, with: event)
      }
      scrub(at: touch.location(in: self))
      guard let index = state.highlightedIndex else { return }
      state.highlightedIndex = nil
      onCommit?(index)
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
      guard isExpanded else { return super.touchesCancelled(touches, with: event) }
      highlight(nil)
    }

    private func scrub(at point: CGPoint) {
      let local = convert(point, to: menuView)
      var index: Int? = nil
      if local.x >= 0, local.x <= menuView.bounds.width, local.y >= 0 {
        let row = Int(local.y / CreateMenu.rowHeight)
        if row < state.items.count { index = row }
      }
      highlight(index)
    }

    private func highlight(_ index: Int?) {
      guard state.highlightedIndex != index else { return }
      state.highlightedIndex = index
      onHighlightChange?(index)
    }

    func setItems(_ items: [CreateMenuItem]) {
      state.items = items
      invalidateIntrinsicContentSize()
      if isExpanded { glassHeight.constant = cardHeight }
    }

    func setExpanded(_ expanded: Bool) {
      guard expanded != isExpanded, isShown || !expanded else { return }
      isExpanded = expanded
      onExpandedChange?(expanded)
      state.isPresented = expanded
      if !expanded { highlight(nil) }
      button.isUserInteractionEnabled = !expanded
      button.accessibilityElementsHidden = expanded
      let reduceMotion = UIAccessibility.isReduceMotionEnabled
      glassWidth.constant = expanded ? Self.cardWidth : Self.side
      glassHeight.constant = expanded ? cardHeight : Self.side
      animateGlyph(
        duration: expanded ? Self.glyphExitDuration : Self.glyphEnterDuration,
        delay: expanded ? 0 : Self.glyphEnterDelay
      ) {
        self.button.alpha = expanded ? 0 : 1
        self.button.transform =
          expanded && !reduceMotion
          ? CGAffineTransform(translationX: 0, y: Self.glyphSlideDistance) : .identity
      }
      UIView.animate(
        springDuration: expanded ? Self.expandDuration : Self.collapseDuration,
        bounce: reduceMotion ? 0 : (expanded ? Self.expandBounce : Self.collapseBounce),
        options: [.allowUserInteraction, .beginFromCurrentState],
        animations: {
          self.glass.effect = expanded ? Self.plainGlass() : Self.glassEffect(self.tint)
          self.layoutIfNeeded()
        })
      if expanded {
        UIAccessibility.post(notification: .layoutChanged, argument: menuView)
      }
    }

    func setShown(_ shown: Bool, duration: TimeInterval? = nil) {
      guard shown != isShown else { return }
      if !shown, isExpanded { setExpanded(false) }
      isShown = shown
      isUserInteractionEnabled = shown
      isHidden = false
      let duration = duration ?? (UIAccessibility.isReduceMotionEnabled ? 0 : Self.fade)
      if duration == 0 {
        UIView.performWithoutAnimation {
          glass.effect = shown ? Self.glassEffect(tint) : nil
          button.alpha = shown ? 1 : 0
        }
        isHidden = !shown
        return
      }
      UIView.animate(
        springDuration: duration, bounce: 0,
        options: [.allowUserInteraction, .beginFromCurrentState],
        animations: {
          self.glass.effect = shown ? Self.glassEffect(self.tint) : nil
          self.button.alpha = shown ? 1 : 0
        }
      ) { finished in
        guard finished, self.isShown == shown, !shown else { return }
        self.isHidden = true
      }
    }

    private func animateGlyph(
      duration: TimeInterval, delay: TimeInterval, animations: @escaping () -> Void
    ) {
      glyphAnimator?.stopAnimation(true)
      let animator = UIViewPropertyAnimator(
        duration: duration,
        timingParameters: UICubicTimingParameters(
          controlPoint1: CGPoint(x: 0.23, y: 1), controlPoint2: CGPoint(x: 0.32, y: 1)))
      animator.addAnimations(animations)
      animator.startAnimation(afterDelay: delay)
      glyphAnimator = animator
    }

    private static func glassEffect(_ tint: UIColor) -> UIGlassEffect {
      let effect = plainGlass()
      effect.tintColor = tint
      return effect
    }

    private static func plainGlass() -> UIGlassEffect {
      let effect = UIGlassEffect()
      effect.isInteractive = true
      return effect
    }
  }

#endif
