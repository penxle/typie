#if canImport(UIKit)

  import Design
  import UIKit
  import UIKit.UIGestureRecognizerSubclass

  final class EditorScrollbar: UIView {
    static let laneWidth = CGFloat(EditorScrollbarLayout.touchExtent)
    private static let slotWidth: CGFloat = 10
    private static let thumbDuration: TimeInterval = 0.25
    private static let thumbCurve = (CGPoint(x: 0.68, y: -0.6), CGPoint(x: 0.32, y: 1.6))

    var layout = EditorScrollbarLayout() {
      didSet {
        guard layout != oldValue else { return }
        isAccessibilityElement = layout.maxScroll > 0
        placeThumb()
      }
    }
    var onGrab: ((Bool) -> Void)?
    var onScroll: ((Double) -> Void)?

    private let model = EditorScrollbarModel()
    private let fader = UIView()
    private let slot = UIView()
    private let thumb = UIView()
    private let grab = EditorScrollbarGrabRecognizer()
    private lazy var haptics = UIImpactFeedbackGenerator(style: .light, view: self)
    private var drag: EditorScrollbarDrag?
    private var shownOpacity: Double = 0
    private var shownThickness: Double = 0
    private var shownThumbOpacity: Double = 0
    private var thumbAnimator: UIViewPropertyAnimator?

    override init(frame: CGRect) {
      super.init(frame: frame)
      isUserInteractionEnabled = false
      accessibilityLabel = "문서 본문 세로 스크롤"
      accessibilityTraits = .adjustable
      fader.alpha = 0
      thumb.backgroundColor = .theme(\.surfaceInverse)
      thumb.autoresizingMask = [.flexibleHeight]
      addSubview(fader)
      fader.addSubview(slot)
      slot.addSubview(thumb)
      grab.scrollbar = self
      grab.addTarget(self, action: #selector(handleGrab))
      model.onChange = { [weak self] in self?.render() }
      shapeThumb(animated: false)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    func install(on host: UIView, over scrollView: UIScrollView) {
      host.addGestureRecognizer(grab)
      scrollView.panGestureRecognizer.require(toFail: grab)
    }

    func scrolled(automatic: Bool) {
      model.scrolled(automatic: automatic)
    }

    func hide() {
      model.hide()
    }

    func cancelGrab() {
      grab.isEnabled = false
      grab.isEnabled = true
    }

    override func layoutSubviews() {
      super.layoutSubviews()
      fader.frame = bounds
      placeThumb()
    }

    override func accessibilityIncrement() {
      page(by: 1)
    }

    override func accessibilityDecrement() {
      page(by: -1)
    }

    fileprivate var canGrab: Bool {
      layout.isVisible && model.canGrab
    }

    fileprivate func accepts(_ touch: UITouch) -> Bool {
      canGrab
        && layout.touchArea(width: bounds.width, thickness: model.thickness).contains(
          touch.location(in: self))
    }

    fileprivate func pressed() {
      haptics.prepare()
    }

    @objc private func handleGrab() {
      switch grab.state {
      case .began:
        model.grabChanged(true)
        haptics.impactOccurred()
        onGrab?(true)
        drag = EditorScrollbarDrag(layout)
        move(by: grab.increment)
      case .changed:
        move(by: grab.increment)
      case .ended, .cancelled:
        guard drag != nil else { return }
        drag = nil
        model.grabChanged(false)
        haptics.impactOccurred()
        onGrab?(false)
      default:
        break
      }
    }

    private func move(by delta: Double) {
      guard var current = drag, let position = current.move(by: delta, in: layout) else { return }
      drag = current
      onScroll?(position)
    }

    private func page(by direction: Double) {
      let target = min(
        max(layout.scrollPosition + direction * layout.visibleHeight, 0), layout.maxScroll)
      guard target != layout.scrollPosition else { return }
      onScroll?(target)
    }

    private func placeThumb() {
      slot.isHidden = !layout.isVisible
      guard layout.isVisible else { return }
      slot.frame = layout.thumbFrame(width: bounds.width, thickness: Self.slotWidth)
    }

    private func render() {
      let animated = window != nil && !UIAccessibility.isReduceMotionEnabled
      let opacity = model.opacity
      if opacity != shownOpacity {
        shownOpacity = opacity
        if animated {
          UIView.animate(
            withDuration: EditorScrollbarModel.fadeDuration, delay: 0,
            options: [.curveLinear, .beginFromCurrentState, .allowUserInteraction],
            animations: { self.fader.alpha = opacity })
        } else {
          UIView.performWithoutAnimation { fader.alpha = opacity }
        }
      }
      shapeThumb(animated: animated)
    }

    private func shapeThumb(animated: Bool) {
      let thickness = model.thickness
      let alpha = model.thumbOpacity
      guard thickness != shownThickness || alpha != shownThumbOpacity else { return }
      shownThickness = thickness
      shownThumbOpacity = alpha
      let changes = {
        self.thumb.frame = CGRect(
          x: Self.slotWidth - thickness, y: 0, width: thickness, height: self.slot.bounds.height)
        self.thumb.layer.cornerRadius = thickness / 2
        self.thumb.alpha = alpha
      }
      thumbAnimator?.stopAnimation(true)
      thumbAnimator = nil
      guard animated else {
        UIView.performWithoutAnimation(changes)
        return
      }
      let animator = UIViewPropertyAnimator(
        duration: Self.thumbDuration, controlPoint1: Self.thumbCurve.0,
        controlPoint2: Self.thumbCurve.1, animations: changes)
      animator.startAnimation()
      thumbAnimator = animator
    }
  }

  private final class EditorScrollbarGrabRecognizer: UIGestureRecognizer {
    weak var scrollbar: EditorScrollbar?
    private(set) var increment: Double = 0
    private var tracked: UITouch?
    private var origin = CGPoint.zero
    private var last = CGPoint.zero
    private var pressedAt: TimeInterval = 0
    private var hold: Task<Void, Never>?

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent) {
      super.touchesBegan(touches, with: event)
      if tracked != nil {
        for touch in touches {
          ignore(touch, for: event)
        }
        return
      }
      guard let touch = touches.first, let scrollbar, scrollbar.accepts(touch) else {
        state = .failed
        return
      }
      for other in touches where other !== touch {
        ignore(other, for: event)
      }
      tracked = touch
      origin = touch.location(in: view)
      last = origin
      pressedAt = touch.timestamp
      scrollbar.pressed()
      hold = Task { [weak self] in
        try? await Task.sleep(for: EditorScrollbarGrab.holdDuration)
        guard !Task.isCancelled else { return }
        self?.held()
      }
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent) {
      super.touchesMoved(touches, with: event)
      guard let tracked, touches.contains(tracked), let scrollbar else { return }
      let point = tracked.location(in: view)
      let delta = Double(point.y - last.y)
      last = point
      switch state {
      case .possible:
        switch EditorScrollbarGrab.decide(
          canGrab: scrollbar.canGrab,
          displacement: CGVector(dx: point.x - origin.x, dy: point.y - origin.y),
          elapsed: .seconds(tracked.timestamp - pressedAt))
        {
        case .pending:
          break
        case .claim:
          increment = delta
          state = .began
        case .yield:
          state = .failed
        }
      case .began, .changed:
        increment = delta
        state = .changed
      default:
        break
      }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent) {
      super.touchesEnded(touches, with: event)
      guard let tracked, touches.contains(tracked) else { return }
      state = state == .began || state == .changed ? .ended : .failed
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent) {
      super.touchesCancelled(touches, with: event)
      guard let tracked, touches.contains(tracked) else { return }
      state = state == .began || state == .changed ? .cancelled : .failed
    }

    override func reset() {
      super.reset()
      hold?.cancel()
      hold = nil
      tracked = nil
      increment = 0
    }

    private func held() {
      hold = nil
      guard state == .possible, tracked != nil, let scrollbar else { return }
      let decision = EditorScrollbarGrab.decide(
        canGrab: scrollbar.canGrab,
        displacement: CGVector(dx: last.x - origin.x, dy: last.y - origin.y),
        elapsed: EditorScrollbarGrab.holdDuration)
      if decision == .claim {
        increment = 0
        state = .began
      } else {
        state = .failed
      }
    }
  }

#endif
