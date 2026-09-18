import Core
import Design
import UIKit

@available(iOS 26, *)
final class MainTabBar: UIView {
  static let barHeight: CGFloat = 54
  static let horizontalPadding: CGFloat = 21
  static let bottomPadding: CGFloat = 21
  static let contentBottomInset = barHeight + bottomPadding
  static let iconSide: CGFloat = 22
  private static let createSpacing: CGFloat = 8
  private static let contentPadding: CGFloat = 2
  private static let lightSelectionAlpha: CGFloat = 0.06
  private static let darkSelectionAlpha: CGFloat = 0.12
  private static let exitSlideDistance: CGFloat = 24
  private static let selectionTint = UIColor { traits in
    traits.userInterfaceStyle == .dark
      ? UIColor.white.withAlphaComponent(darkSelectionAlpha)
      : UIColor.black.withAlphaComponent(lightSelectionAlpha)
  }
  private static let iconExitDuration: TimeInterval = 0.12
  private static let iconEnterDuration: TimeInterval = 0.2
  private static let iconEnterDelay: TimeInterval = 0.1
  private static let expandDuration: TimeInterval = 0.35
  private static let expandBounce: CGFloat = 0.3
  private static let collapseDuration: TimeInterval = 0.3
  private static let collapseBounce: CGFloat = 0.15
  private static let menuPadding: CGFloat = 8

  private let containerView: UIVisualEffectView
  private let tabGlassView: UIVisualEffectView
  private let pillView: UIVisualEffectView
  private let menuGlassView: UIVisualEffectView
  private let createView: UIVisualEffectView
  private let tabContentView = UIView()
  private let segmentedControl: TabSegmentedControl
  private let iconsOverlay: TabIconsOverlay
  private let menuView: UIView
  private let menuHeaderHeight: CGFloat
  private let menuRowHeight: CGFloat
  private let menuHeight: CGFloat
  private var didConfigureCorners = false
  private var pillHeight: NSLayoutConstraint?
  private var iconAnimator: UIViewPropertyAnimator?
  private var isIgnoringTouch = false
  private var settleLink: CADisplayLink?
  private var lastSettleHeight: CGFloat?
  let createButton = UIButton(type: .system)
  private(set) var isExpanded = false

  var onSelect: ((MainTab) -> Void)?
  var onExpandedChange: ((Bool) -> Void)?
  var onScrubHighlight: ((Int?) -> Void)?
  var onScrubCommit: (() -> Void)?

  init(
    selectedTint: UIColor, normalTint: UIColor, accentTint: UIColor, menuView: UIView,
    menuHeaderHeight: CGFloat, menuRowHeight: CGFloat, menuRowCount: Int
  ) {
    self.menuView = menuView
    self.menuHeaderHeight = menuHeaderHeight
    self.menuRowHeight = menuRowHeight
    self.menuHeight = menuHeaderHeight + menuRowHeight * CGFloat(menuRowCount)
    let containerEffect = UIGlassContainerEffect()
    containerEffect.spacing = Self.createSpacing
    containerView = UIVisualEffectView(effect: containerEffect)
    tabGlassView = UIVisualEffectView(effect: Self.interactiveGlass())
    pillView = UIVisualEffectView(effect: UIGlassEffect())
    menuGlassView = UIVisualEffectView(effect: Self.interactiveGlass())
    createView = UIVisualEffectView(effect: Self.interactiveGlass())

    let switcherImage = UIImage(
      named: LucideIcon.chevronsUpDown.assetName, in: DesignBundle.bundle, with: nil)
    let images = MainTab.allCases.map(\.selectedImage) + [switcherImage]
    segmentedControl = TabSegmentedControl(
      placeholders: MainTab.allCases.map { Self.placeholder(for: $0) }
        + [Self.placeholder(size: switcherImage?.size)],
      momentaryIndex: MainTab.allCases.count)
    iconsOverlay = TabIconsOverlay(
      images: images, normalTint: normalTint, accentTint: accentTint, iconSide: Self.iconSide)
    super.init(frame: .zero)

    segmentedControl.selectedSegmentIndex = 0
    segmentedControl.backgroundColor = .clear
    segmentedControl.selectedSegmentTintColor = Self.selectionTint
    segmentedControl.accessibilityTraits = .tabBar
    iconsOverlay.lensProvider = { [weak segmentedControl] in segmentedControl?.lensView }
    segmentedControl.onLensMayMove = { [weak iconsOverlay] in iconsOverlay?.resumeTracking() }
    segmentedControl.onMomentary = { [weak self] in
      self?.setExpanded(true)
    }
    iconsOverlay.selectedIndexProvider = { [weak segmentedControl] in
      segmentedControl?.selectedSegmentIndex ?? 0
    }
    segmentedControl.addAction(
      UIAction { [weak self] _ in
        guard let self, MainTab.allCases.indices.contains(segmentedControl.selectedSegmentIndex)
        else { return }
        onSelect?(MainTab.allCases[segmentedControl.selectedSegmentIndex])
      }, for: .valueChanged)

    createButton.configuration = .plain()
    createButton.configuration?.baseForegroundColor = selectedTint

    let content = containerView.contentView
    addSubview(containerView)
    content.addSubview(tabGlassView)
    content.addSubview(pillView)
    pillView.isHidden = true
    pillView.contentView.clipsToBounds = true
    content.addSubview(menuGlassView)
    menuGlassView.isHidden = true
    menuGlassView.contentView.clipsToBounds = true
    menuView.backgroundColor = .clear
    menuView.isUserInteractionEnabled = false
    pillView.contentView.addSubview(menuView)
    tabGlassView.contentView.addSubview(tabContentView)
    tabContentView.addSubview(segmentedControl)
    tabContentView.addSubview(iconsOverlay)
    content.addSubview(createView)
    createView.contentView.addSubview(createButton)
    for view in [
      containerView, tabGlassView, pillView, menuGlassView, segmentedControl, iconsOverlay,
      createView, createButton,
    ] {
      view.translatesAutoresizingMaskIntoConstraints = false
    }

    let createContent = createView.contentView
    let pillHeight = pillView.heightAnchor.constraint(equalToConstant: Self.barHeight)
    self.pillHeight = pillHeight
    NSLayoutConstraint.activate([
      pillHeight,
      containerView.heightAnchor.constraint(equalToConstant: expandedHeight),
      containerView.leadingAnchor.constraint(equalTo: leadingAnchor),
      containerView.trailingAnchor.constraint(equalTo: trailingAnchor),
      containerView.topAnchor.constraint(equalTo: topAnchor),
      containerView.bottomAnchor.constraint(equalTo: bottomAnchor),

      pillView.leadingAnchor.constraint(equalTo: content.leadingAnchor),
      pillView.bottomAnchor.constraint(equalTo: content.bottomAnchor),
      pillView.trailingAnchor.constraint(
        equalTo: createView.leadingAnchor, constant: -Self.createSpacing),

      tabGlassView.leadingAnchor.constraint(equalTo: pillView.leadingAnchor),
      tabGlassView.trailingAnchor.constraint(equalTo: pillView.trailingAnchor),
      tabGlassView.bottomAnchor.constraint(equalTo: content.bottomAnchor),
      tabGlassView.heightAnchor.constraint(equalToConstant: Self.barHeight),

      menuGlassView.leadingAnchor.constraint(equalTo: pillView.leadingAnchor),
      menuGlassView.trailingAnchor.constraint(equalTo: pillView.trailingAnchor),
      menuGlassView.bottomAnchor.constraint(equalTo: content.bottomAnchor),
      menuGlassView.heightAnchor.constraint(equalToConstant: expandedHeight),

      segmentedControl.leadingAnchor.constraint(
        equalTo: tabContentView.leadingAnchor, constant: Self.contentPadding),
      segmentedControl.trailingAnchor.constraint(
        equalTo: tabContentView.trailingAnchor, constant: -Self.contentPadding),
      segmentedControl.topAnchor.constraint(
        equalTo: tabContentView.topAnchor, constant: Self.contentPadding),
      segmentedControl.bottomAnchor.constraint(
        equalTo: tabContentView.bottomAnchor, constant: -Self.contentPadding),

      iconsOverlay.leadingAnchor.constraint(equalTo: segmentedControl.leadingAnchor),
      iconsOverlay.trailingAnchor.constraint(equalTo: segmentedControl.trailingAnchor),
      iconsOverlay.topAnchor.constraint(equalTo: segmentedControl.topAnchor),
      iconsOverlay.bottomAnchor.constraint(equalTo: segmentedControl.bottomAnchor),

      createView.trailingAnchor.constraint(equalTo: content.trailingAnchor),
      createView.bottomAnchor.constraint(equalTo: content.bottomAnchor),
      createView.widthAnchor.constraint(equalToConstant: Self.barHeight),
      createView.heightAnchor.constraint(equalToConstant: Self.barHeight),

      createButton.leadingAnchor.constraint(equalTo: createContent.leadingAnchor),
      createButton.trailingAnchor.constraint(equalTo: createContent.trailingAnchor),
      createButton.topAnchor.constraint(equalTo: createContent.topAnchor),
      createButton.bottomAnchor.constraint(equalTo: createContent.bottomAnchor),
    ])
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    nil
  }

  override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
    let surface = isExpanded ? menuGlassView : tabGlassView
    let insideSurface = surface.point(inside: convert(point, to: surface), with: event)
    let insideCreate = createView.point(inside: convert(point, to: createView), with: event)
    guard insideSurface || insideCreate else { return nil }
    return super.hitTest(point, with: event)
  }

  override func layoutSubviews() {
    super.layoutSubviews()
    if !didConfigureCorners {
      didConfigureCorners = true
      tabGlassView.cornerConfiguration = .capsule(maximumRadius: Self.barHeight / 2)
      pillView.cornerConfiguration = .capsule(maximumRadius: Self.barHeight / 2)
      menuGlassView.cornerConfiguration = .capsule(maximumRadius: Self.barHeight / 2)
      createView.cornerConfiguration = .capsule()
    }
    menuView.frame = CGRect(
      x: Self.menuPadding, y: Self.menuPadding, width: max(0, pillWidth - Self.menuPadding * 2),
      height: menuHeight)
    layoutTabContent()
  }

  private func layoutTabContent() {
    let insideGlass = tabContentView.superview === tabGlassView.contentView
    tabContentView.frame = CGRect(
      x: 0, y: insideGlass ? 0 : expandedHeight - Self.barHeight, width: max(0, pillWidth),
      height: Self.barHeight)
  }

  private func hostTabContent(inGlass: Bool) {
    UIView.performWithoutAnimation {
      let host = inGlass ? tabGlassView.contentView : containerView.contentView
      if tabContentView.superview !== host {
        host.addSubview(tabContentView)
      }
      layoutTabContent()
      tabGlassView.isHidden = !inGlass
      if !inGlass, pillView.isHidden, menuGlassView.isHidden {
        pillHeight?.constant = Self.barHeight
        pillView.isHidden = false
        superview?.layoutIfNeeded()
      }
      if inGlass {
        pillView.isHidden = true
      }
    }
  }

  override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
    guard isExpanded, let touch = touches.first else {
      super.touchesBegan(touches, with: event)
      return
    }
    guard !menuGlassView.isHidden else {
      isIgnoringTouch = true
      return
    }
    scrub(at: touch.location(in: self), in: self)
  }

  override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
    guard isExpanded, let touch = touches.first else {
      super.touchesMoved(touches, with: event)
      return
    }
    guard !isIgnoringTouch else { return }
    scrub(at: touch.location(in: self), in: self)
  }

  override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
    guard isExpanded, let touch = touches.first else {
      isIgnoringTouch = false
      super.touchesEnded(touches, with: event)
      return
    }
    guard !isIgnoringTouch else {
      isIgnoringTouch = false
      return
    }
    scrub(at: touch.location(in: self), in: self)
    onScrubCommit?()
  }

  override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
    guard isExpanded else {
      isIgnoringTouch = false
      super.touchesCancelled(touches, with: event)
      return
    }
    guard !isIgnoringTouch else {
      isIgnoringTouch = false
      return
    }
    onScrubHighlight?(nil)
  }

  private func scrub(at point: CGPoint, in source: UIView?) {
    let local = menuView.convert(point, from: source)
    var index: Int?
    if menuView.bounds.insetBy(dx: -Self.menuPadding, dy: 0).contains(local) {
      let rowsY = local.y - menuHeaderHeight
      let row = Int(floor(rowsY / menuRowHeight))
      if rowsY >= 0, menuHeaderHeight + CGFloat(row) * menuRowHeight < menuHeight {
        index = row
      }
    }
    onScrubHighlight?(index)
  }

  func setExpanded(_ expanded: Bool) {
    guard expanded != isExpanded else { return }
    isExpanded = expanded
    onExpandedChange?(expanded)
    tabContentView.isUserInteractionEnabled = !expanded
    let tabLayers = [segmentedControl, iconsOverlay]
    let reduceMotion = UIAccessibility.isReduceMotionEnabled
    if expanded {
      hostTabContent(inGlass: false)
      iconsOverlay.updateNow()
      iconsOverlay.pauseTracking()
      animateIcons(duration: Self.iconExitDuration, delay: 0) {
        for layer in tabLayers {
          layer.alpha = 0
          layer.transform = reduceMotion ? .identity : self.suckedTransform
        }
      }
      UIView.animate(
        springDuration: Self.expandDuration, bounce: reduceMotion ? 0 : Self.expandBounce,
        options: [.allowUserInteraction],
        animations: {
          self.pillHeight?.constant = self.expandedHeight
          self.superview?.layoutIfNeeded()
        }
      ) { _ in
        guard self.isExpanded else { return }
        self.handOff(toInteractive: true)
      }
      startSettleLink()
    } else {
      handOff(toInteractive: false)
      segmentedControl.selectedSegmentTintColor = Self.selectionTint
      segmentedControl.setLensHidden(false)
      iconsOverlay.resumeTracking()
      animateIcons(duration: Self.iconEnterDuration, delay: Self.iconEnterDelay) {
        for layer in tabLayers {
          layer.alpha = 1
          layer.transform = .identity
        }
      }
      UIView.animate(
        springDuration: Self.collapseDuration, bounce: reduceMotion ? 0 : Self.collapseBounce,
        options: [.allowUserInteraction],
        animations: {
          self.pillHeight?.constant = Self.barHeight
          self.superview?.layoutIfNeeded()
        }
      ) { _ in
        guard !self.isExpanded, !self.segmentedControl.isTracking else { return }
        self.hostTabContent(inGlass: true)
      }
    }
  }

  private func startSettleLink() {
    stopSettleLink()
    let link = CADisplayLink(target: self, selector: #selector(checkSettled))
    link.add(to: .main, forMode: .common)
    settleLink = link
  }

  private func stopSettleLink() {
    settleLink?.invalidate()
    settleLink = nil
    lastSettleHeight = nil
  }

  @objc private func checkSettled() {
    guard isExpanded, let height = pillView.layer.presentation()?.bounds.height else {
      stopSettleLink()
      return
    }
    let pixel = 1 / max(traitCollection.displayScale, 1)
    let previous = lastSettleHeight
    lastSettleHeight = height
    guard let previous, abs(height - expandedHeight) < pixel, abs(height - previous) < pixel
    else { return }
    handOff(toInteractive: true)
  }

  private func handOff(toInteractive interactive: Bool) {
    stopSettleLink()
    UIView.performWithoutAnimation {
      let host = interactive ? menuGlassView : pillView
      if menuView.superview !== host.contentView {
        host.contentView.addSubview(menuView)
      }
      menuGlassView.isHidden = !interactive
      pillView.isHidden = interactive
    }
  }

  private func animateIcons(
    duration: TimeInterval, delay: TimeInterval, animations: @escaping () -> Void
  ) {
    iconAnimator?.stopAnimation(true)
    let animator = UIViewPropertyAnimator(
      duration: duration,
      timingParameters: UICubicTimingParameters(
        controlPoint1: CGPoint(x: 0.23, y: 1), controlPoint2: CGPoint(x: 0.32, y: 1)))
    animator.addAnimations(animations)
    animator.startAnimation(afterDelay: delay)
    iconAnimator = animator
  }

  private var pillWidth: CGFloat {
    bounds.width - Self.createSpacing - Self.barHeight
  }

  private var expandedHeight: CGFloat {
    Self.menuPadding * 2 + menuHeight
  }

  private var suckedTransform: CGAffineTransform {
    CGAffineTransform(translationX: 0, y: Self.exitSlideDistance)
  }

  private static func interactiveGlass() -> UIGlassEffect {
    let effect = UIGlassEffect()
    effect.isInteractive = true
    return effect
  }

  private static func placeholder(for tab: MainTab) -> UIImage {
    let image = placeholder(size: tab.image?.size)
    image.accessibilityLabel = tab.label
    return image
  }

  private static func placeholder(size: CGSize?) -> UIImage {
    UIGraphicsImageRenderer(size: size ?? CGSize(width: 24, height: 24)).image { _ in }
  }
}

@available(iOS 26, *)
private final class TabIconsOverlay: UIView {
  private static let stableFrameThreshold = 3

  private let iconSide: CGFloat
  private let baseViews: [UIImageView]
  private let accentViews: [UIImageView]
  private var displayLink: CADisplayLink?
  private var lastLensRect: CGRect = .null
  private var stableFrames = 0

  var lensProvider: (() -> UIView?)?
  var selectedIndexProvider: (() -> Int)?

  init(images: [UIImage?], normalTint: UIColor, accentTint: UIColor, iconSide: CGFloat) {
    self.iconSide = iconSide
    baseViews = images.map { image in
      let view = UIImageView(image: image?.withRenderingMode(.alwaysTemplate))
      view.contentMode = .scaleAspectFit
      view.tintColor = normalTint
      return view
    }
    accentViews = images.map { image in
      let view = UIImageView(image: image?.withRenderingMode(.alwaysTemplate))
      view.contentMode = .scaleAspectFit
      view.tintColor = accentTint
      view.layer.mask = CAShapeLayer()
      return view
    }
    super.init(frame: .zero)
    isUserInteractionEnabled = false
    for view in baseViews + accentViews {
      addSubview(view)
    }
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    nil
  }

  override func layoutSubviews() {
    super.layoutSubviews()
    let count = CGFloat(baseViews.count)
    guard count > 0 else { return }
    let segmentWidth = bounds.width / count
    for (index, base) in baseViews.enumerated() {
      let center = CGPoint(x: segmentWidth * (CGFloat(index) + 0.5), y: bounds.midY)
      base.bounds = CGRect(origin: .zero, size: CGSize(width: iconSide, height: iconSide))
      base.center = center
      accentViews[index].bounds = base.bounds
      accentViews[index].center = center
    }
    resumeTracking()
  }

  override func didMoveToWindow() {
    super.didMoveToWindow()
    if window != nil {
      startDisplayLink()
    } else {
      stopDisplayLink()
    }
  }

  func resumeTracking() {
    stableFrames = 0
    displayLink?.isPaused = false
  }

  func pauseTracking() {
    displayLink?.isPaused = true
  }

  func updateNow() {
    lastLensRect = .null
    step()
  }

  private func startDisplayLink() {
    guard displayLink == nil else { return }
    let link = CADisplayLink(target: self, selector: #selector(step))
    link.add(to: .main, forMode: .common)
    displayLink = link
  }

  private func stopDisplayLink() {
    displayLink?.invalidate()
    displayLink = nil
  }

  private func currentLensRect() -> CGRect {
    if let lens = lensProvider?(), let lensSuperlayer = lens.layer.superlayer {
      let presentation = lens.layer.presentation() ?? lens.layer
      let rect = lensSuperlayer.convert(presentation.frame, to: layer)
      return rect
    }
    let index = selectedIndexProvider?() ?? 0
    guard baseViews.indices.contains(index) else { return .null }
    let segmentWidth = bounds.width / CGFloat(baseViews.count)
    return CGRect(
      x: segmentWidth * CGFloat(index), y: 0, width: segmentWidth, height: bounds.height)
  }

  @objc private func step() {
    let lensRect = currentLensRect()
    if lensRect == lastLensRect {
      stableFrames += 1
      if stableFrames >= Self.stableFrameThreshold {
        displayLink?.isPaused = true
      }
      return
    }
    stableFrames = 0
    lastLensRect = lensRect
    CATransaction.begin()
    CATransaction.setDisableActions(true)
    for (index, accent) in accentViews.enumerated() {
      let local = accent.convert(lensRect, from: self)
      let capsule = UIBezierPath(roundedRect: local, cornerRadius: local.height / 2)
      (accent.layer.mask as? CAShapeLayer)?.path = capsule.cgPath
      let base = baseViews[index]
      if lensRect.intersects(base.frame) {
        let cutout = UIBezierPath(rect: base.bounds)
        cutout.append(
          UIBezierPath(
            roundedRect: base.convert(lensRect, from: self), cornerRadius: local.height / 2))
        let mask = base.layer.mask as? CAShapeLayer ?? CAShapeLayer()
        mask.fillRule = .evenOdd
        mask.path = cutout.cgPath
        base.layer.mask = mask
      } else {
        base.layer.mask = nil
      }
    }
    CATransaction.commit()
  }
}

@available(iOS 26, *)
private final class TabSegmentedControl: UISegmentedControl {
  private static let momentaryHoldDelay: TimeInterval = 0.2

  private let momentaryIndex: Int
  private var originalIndex: Int?
  private var pendingMomentaryTap = false
  private var momentaryHoldTimer: Timer?
  private var pendingTouches: Set<UITouch> = []
  private weak var pendingEvent: UIEvent?
  private weak var cachedLensView: UIView?
  var onLensMayMove: (() -> Void)?
  var onMomentary: (() -> Void)?

  override var selectedSegmentIndex: Int {
    didSet { onLensMayMove?() }
  }

  init(placeholders: [UIImage], momentaryIndex: Int) {
    self.momentaryIndex = momentaryIndex
    super.init(items: placeholders)
  }

  var lensView: UIView? {
    if let cachedLensView { return cachedLensView }
    let found = Self.findLens(in: self)
    cachedLensView = found
    return found
  }

  func setLensHidden(_ hidden: Bool) {
    guard let lens = lensView else { return }
    lens.isHidden = hidden
    for key in ["liftedContainerView", "liftedContentView"] {
      (lens.value(forKey: key) as? UIView)?.isHidden = hidden
    }
  }

  private func forceInstantUnlift() {
    guard let lens = lensView else { return }
    lens.setValue(true, forKey: "forceUnliftTimerWithoutAnimations")
    if let timer = lens.value(forKey: "unliftDelayTimer") as? Timer, timer.isValid {
      timer.fire()
    }
    lens.setValue(false, forKey: "forceUnliftTimerWithoutAnimations")
  }

  private static func findLens(in view: UIView) -> UIView? {
    for subview in view.subviews {
      if String(describing: type(of: subview)) == "_UILiquidLensView" { return subview }
      if let found = findLens(in: subview) { return found }
    }
    return nil
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    nil
  }

  override func layoutSubviews() {
    super.layoutSubviews()
    for subview in subviews where subview is UIImageView {
      subview.alpha = 0
    }
  }

  private func segmentIndex(at point: CGPoint) -> Int {
    guard numberOfSegments > 0 else { return 0 }
    let segmentWidth = bounds.width / CGFloat(numberOfSegments)
    return min(max(Int(point.x / segmentWidth), 0), numberOfSegments - 1)
  }

  override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
    onLensMayMove?()
    if let touch = touches.first {
      originalIndex = selectedSegmentIndex
      let index = segmentIndex(at: touch.location(in: self))
      if index == momentaryIndex {
        pendingMomentaryTap = true
        pendingTouches = touches
        pendingEvent = event
        momentaryHoldTimer = Timer.scheduledTimer(
          withTimeInterval: Self.momentaryHoldDelay, repeats: false
        ) { [weak self] _ in
          MainActor.assumeIsolated { self?.liftLensOnMomentary() }
        }
      } else {
        selectedSegmentIndex = index
      }
    }
    super.touchesBegan(touches, with: event)
  }

  private func liftLensOnMomentary() {
    guard pendingMomentaryTap else { return }
    let touches = pendingTouches
    let event = pendingEvent
    cancelMomentaryTap()
    super.touchesCancelled(touches, with: event)
    selectedSegmentIndex = momentaryIndex
    super.touchesBegan(touches, with: event)
  }

  private func cancelMomentaryTap() {
    momentaryHoldTimer?.invalidate()
    momentaryHoldTimer = nil
    pendingMomentaryTap = false
    pendingTouches = []
    pendingEvent = nil
  }

  override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
    onLensMayMove?()
    if let touch = touches.first {
      let index = segmentIndex(at: touch.location(in: self))
      if pendingMomentaryTap {
        guard index != momentaryIndex else { return }
        cancelMomentaryTap()
      }
      if selectedSegmentIndex != index {
        selectedSegmentIndex = index
      }
    }
    super.touchesMoved(touches, with: event)
  }

  override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
    if pendingMomentaryTap {
      cancelMomentaryTap()
      originalIndex = nil
      super.touchesCancelled(touches, with: event)
      selectedSegmentTintColor = .clear
      setLensHidden(true)
      onMomentary?()
      return
    }
    onLensMayMove?()
    let landedIndex = selectedSegmentIndex
    let startIndex = originalIndex
    originalIndex = nil
    super.touchesEnded(touches, with: event)
    guard let startIndex, landedIndex != startIndex else { return }
    if landedIndex == momentaryIndex {
      selectedSegmentTintColor = .clear
      forceInstantUnlift()
      UIView.performWithoutAnimation {
        selectedSegmentIndex = startIndex
        layoutIfNeeded()
      }
      setLensHidden(true)
      onMomentary?()
    } else {
      sendActions(for: .valueChanged)
    }
  }

  override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
    if pendingMomentaryTap {
      cancelMomentaryTap()
      originalIndex = nil
      super.touchesCancelled(touches, with: event)
      return
    }
    onLensMayMove?()
    if let originalIndex {
      selectedSegmentIndex = originalIndex
    }
    originalIndex = nil
    super.touchesCancelled(touches, with: event)
  }
}
