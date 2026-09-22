#if canImport(UIKit)

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
    private static let selectionTint = UIColor { traits in
      traits.userInterfaceStyle == .dark
        ? UIColor.white.withAlphaComponent(darkSelectionAlpha)
        : UIColor.black.withAlphaComponent(lightSelectionAlpha)
    }
    private static let searchExpandDuration: TimeInterval = 0.25
    private static let searchExpandBounce: CGFloat = 0
    private static let searchCollapseDuration: TimeInterval = 0.2
    private static let searchCollapseBounce: CGFloat = 0
    private static let collapsedTabScale: CGFloat = 0.01
    private static let dismissEntryTransform = CGAffineTransform(scaleX: 0.6, y: 0.6)
    private static let progressDelay: TimeInterval = 0.25
    private static let progressCrossfade: TimeInterval = 0.15
    private static let concealBlurRadius: CGFloat = 12
    private static let concealScale: CGFloat = 1.15

    private let containerView: UIVisualEffectView
    private let tabGlassView: UIVisualEffectView
    private let createView: UIVisualEffectView
    private let tabContentView = UIView()
    private let segmentedControl: TabSegmentedControl
    private let iconsOverlay: TabIconsOverlay
    private var didConfigureCorners = false
    private let dismissView: UIVisualEffectView
    private let searchFieldView: UIView
    private let progressView: UIView
    private var pendingProgress: DispatchWorkItem?
    let searchButton = UIButton(type: .system)
    let dismissButton = UIButton(type: .system)
    private(set) var isSearching = false
    private(set) var isConcealed = false
    private var tabFormConstraints: [NSLayoutConstraint] = []
    private var searchFormConstraints: [NSLayoutConstraint] = []

    var onSelect: ((MainTab) -> Void)?

    init(
      selectedTint: UIColor, normalTint: UIColor, accentTint: UIColor, searchFieldView: UIView,
      progressView: UIView
    ) {
      self.searchFieldView = searchFieldView
      self.progressView = progressView
      let containerEffect = UIGlassContainerEffect()
      containerEffect.spacing = Self.createSpacing
      containerView = UIVisualEffectView(effect: containerEffect)
      tabGlassView = UIVisualEffectView(effect: Self.interactiveGlass())
      createView = UIVisualEffectView(effect: Self.interactiveGlass())
      dismissView = UIVisualEffectView(effect: Self.interactiveGlass())

      let images = MainTab.allCases.map(\.selectedImage)
      let control = TabSegmentedControl(
        placeholders: MainTab.allCases.map { Self.placeholder(for: $0) })
      segmentedControl = control
      iconsOverlay = TabIconsOverlay(
        images: images, normalTint: normalTint, accentTint: accentTint, iconSide: Self.iconSide,
        selectedIndexProvider: { control.selectedSegmentIndex })
      super.init(frame: .zero)

      segmentedControl.selectedSegmentIndex = MainTab.initial.index
      segmentedControl.backgroundColor = .clear
      segmentedControl.selectedSegmentTintColor = Self.selectionTint
      segmentedControl.accessibilityTraits = .tabBar
      iconsOverlay.lensProvider = { [weak segmentedControl] in segmentedControl?.lensView }
      segmentedControl.onLensMayMove = { [weak iconsOverlay] in iconsOverlay?.resumeTracking() }
      segmentedControl.addAction(
        UIAction { [weak self] _ in
          guard let self, MainTab.allCases.indices.contains(segmentedControl.selectedSegmentIndex)
          else { return }
          onSelect?(MainTab.allCases[segmentedControl.selectedSegmentIndex])
        }, for: .valueChanged)

      searchButton.configuration = .plain()
      searchButton.configuration?.baseForegroundColor = selectedTint
      searchButton.configuration?.image = Self.barIcon(LucideIcon.search)
      searchButton.accessibilityLabel = "검색"
      dismissButton.configuration = .plain()
      dismissButton.configuration?.baseForegroundColor = selectedTint
      dismissButton.configuration?.image = Self.barIcon(LucideIcon.x)
      dismissButton.accessibilityLabel = "검색 닫기"

      let content = containerView.contentView
      addSubview(containerView)
      content.addSubview(tabGlassView)
      tabGlassView.contentView.addSubview(tabContentView)
      tabContentView.addSubview(segmentedControl)
      tabContentView.addSubview(iconsOverlay)
      content.addSubview(createView)
      createView.contentView.addSubview(searchButton)
      searchFieldView.backgroundColor = .clear
      searchFieldView.isHidden = true
      searchFieldView.alpha = 0
      createView.contentView.addSubview(searchFieldView)
      progressView.backgroundColor = .clear
      progressView.isUserInteractionEnabled = false
      progressView.isHidden = true
      progressView.alpha = 0
      createView.contentView.addSubview(progressView)
      content.addSubview(dismissView)
      dismissView.isHidden = true
      dismissView.alpha = 0
      dismissView.contentView.addSubview(dismissButton)
      for view in [
        containerView, tabGlassView, segmentedControl, iconsOverlay, createView, searchButton,
        searchFieldView, progressView, dismissView, dismissButton, tabContentView,
      ] {
        view.translatesAutoresizingMaskIntoConstraints = false
      }

      let createContent = createView.contentView
      let tabContent = tabGlassView.contentView
      NSLayoutConstraint.activate([
        containerView.heightAnchor.constraint(equalToConstant: Self.barHeight),
        containerView.leadingAnchor.constraint(equalTo: leadingAnchor),
        containerView.trailingAnchor.constraint(equalTo: trailingAnchor),
        containerView.topAnchor.constraint(equalTo: topAnchor),
        containerView.bottomAnchor.constraint(equalTo: bottomAnchor),

        tabGlassView.leadingAnchor.constraint(equalTo: content.leadingAnchor),
        tabGlassView.trailingAnchor.constraint(
          equalTo: content.trailingAnchor, constant: -(Self.createSpacing + Self.barHeight)),
        tabGlassView.bottomAnchor.constraint(equalTo: content.bottomAnchor),
        tabGlassView.heightAnchor.constraint(equalToConstant: Self.barHeight),

        tabContentView.leadingAnchor.constraint(equalTo: tabContent.leadingAnchor),
        tabContentView.trailingAnchor.constraint(equalTo: tabContent.trailingAnchor),
        tabContentView.topAnchor.constraint(equalTo: tabContent.topAnchor),
        tabContentView.bottomAnchor.constraint(equalTo: tabContent.bottomAnchor),

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

        createView.bottomAnchor.constraint(equalTo: content.bottomAnchor),
        createView.heightAnchor.constraint(equalToConstant: Self.barHeight),

        dismissView.trailingAnchor.constraint(equalTo: content.trailingAnchor),
        dismissView.bottomAnchor.constraint(equalTo: content.bottomAnchor),
        dismissView.widthAnchor.constraint(equalToConstant: Self.barHeight),
        dismissView.heightAnchor.constraint(equalToConstant: Self.barHeight),

        dismissButton.leadingAnchor.constraint(equalTo: dismissView.contentView.leadingAnchor),
        dismissButton.trailingAnchor.constraint(equalTo: dismissView.contentView.trailingAnchor),
        dismissButton.topAnchor.constraint(equalTo: dismissView.contentView.topAnchor),
        dismissButton.bottomAnchor.constraint(equalTo: dismissView.contentView.bottomAnchor),

        searchButton.leadingAnchor.constraint(equalTo: createContent.leadingAnchor),
        searchButton.topAnchor.constraint(equalTo: createContent.topAnchor),
        searchButton.bottomAnchor.constraint(equalTo: createContent.bottomAnchor),
        searchButton.widthAnchor.constraint(equalToConstant: Self.barHeight),
        progressView.leadingAnchor.constraint(equalTo: searchButton.leadingAnchor),
        progressView.trailingAnchor.constraint(equalTo: searchButton.trailingAnchor),
        progressView.topAnchor.constraint(equalTo: searchButton.topAnchor),
        progressView.bottomAnchor.constraint(equalTo: searchButton.bottomAnchor),

        searchFieldView.topAnchor.constraint(equalTo: createContent.topAnchor),
        searchFieldView.bottomAnchor.constraint(equalTo: createContent.bottomAnchor),
      ])

      tabFormConstraints = [
        createView.trailingAnchor.constraint(equalTo: content.trailingAnchor),
        createView.widthAnchor.constraint(equalToConstant: Self.barHeight),
      ]
      searchFormConstraints = [
        createView.leadingAnchor.constraint(equalTo: content.leadingAnchor),
        createView.trailingAnchor.constraint(
          equalTo: dismissView.leadingAnchor, constant: -Self.createSpacing),
        searchFieldView.leadingAnchor.constraint(
          equalTo: createContent.leadingAnchor, constant: Self.barHeight),
        searchFieldView.trailingAnchor.constraint(
          equalTo: createContent.trailingAnchor,
          constant: -(Self.barHeight - TSearchField.clearButtonWidth) / 2),
      ]
      NSLayoutConstraint.activate(tabFormConstraints)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
      let insideTabs =
        !isSearching && tabGlassView.point(inside: convert(point, to: tabGlassView), with: event)
      let insideCreate = createView.point(inside: convert(point, to: createView), with: event)
      let insideDismiss =
        isSearching && dismissView.point(inside: convert(point, to: dismissView), with: event)
      guard insideTabs || insideCreate || insideDismiss else { return nil }
      return super.hitTest(point, with: event)
    }

    override func layoutSubviews() {
      super.layoutSubviews()
      if !didConfigureCorners {
        didConfigureCorners = true
        tabGlassView.cornerConfiguration = .capsule(maximumRadius: Self.barHeight / 2)
        createView.cornerConfiguration = .capsule()
        dismissView.cornerConfiguration = .capsule()
      }
    }

    func setSearchLoading(_ loading: Bool) {
      pendingProgress?.cancel()
      pendingProgress = nil
      guard loading else {
        showProgress(false)
        return
      }
      let item = DispatchWorkItem { [weak self] in self?.showProgress(true) }
      pendingProgress = item
      DispatchQueue.main.asyncAfter(deadline: .now() + Self.progressDelay, execute: item)
    }

    private func showProgress(_ shown: Bool) {
      guard shown != !progressView.isHidden else { return }
      if shown { progressView.isHidden = false }
      UIView.animate(
        withDuration: Self.progressCrossfade,
        animations: {
          self.progressView.alpha = shown ? 1 : 0
          self.searchButton.alpha = shown ? 0 : 1
        }
      ) { _ in
        if !shown, self.progressView.alpha == 0 { self.progressView.isHidden = true }
      }
    }

    func setSearching(_ searching: Bool, completion: (() -> Void)? = nil) {
      guard searching != isSearching else {
        completion?()
        return
      }
      isSearching = searching
      tabContentView.isUserInteractionEnabled = !searching
      searchButton.isUserInteractionEnabled = !searching
      searchButton.accessibilityElementsHidden = searching
      let reduceMotion = UIAccessibility.isReduceMotionEnabled
      let tabSurfaces: [UIView] = [tabGlassView, tabContentView]
      if searching {
        iconsOverlay.pauseTracking()
        dismissView.isHidden = false
        searchFieldView.isHidden = false
        dismissView.transform = reduceMotion ? .identity : Self.dismissEntryTransform
        NSLayoutConstraint.deactivate(tabFormConstraints)
        NSLayoutConstraint.activate(searchFormConstraints)
        UIView.animate(
          springDuration: Self.searchExpandDuration,
          bounce: reduceMotion ? 0 : Self.searchExpandBounce, options: [.allowUserInteraction],
          animations: {
            self.superview?.layoutIfNeeded()
            for surface in tabSurfaces {
              surface.alpha = 0
            }
            self.tabGlassView.transform = reduceMotion ? .identity : self.collapsedTabTransform
            self.dismissView.alpha = 1
            self.dismissView.transform = .identity
            self.searchFieldView.alpha = 1
          }
        ) { _ in
          guard self.isSearching else { return }
          completion?()
        }
      } else {
        NSLayoutConstraint.deactivate(searchFormConstraints)
        NSLayoutConstraint.activate(tabFormConstraints)
        UIView.animate(
          springDuration: Self.searchCollapseDuration,
          bounce: reduceMotion ? 0 : Self.searchCollapseBounce, options: [.allowUserInteraction],
          animations: {
            self.superview?.layoutIfNeeded()
            for surface in tabSurfaces {
              surface.alpha = 1
            }
            self.tabGlassView.transform = .identity
            self.dismissView.alpha = 0
            self.dismissView.transform = reduceMotion ? .identity : Self.dismissEntryTransform
            self.searchFieldView.alpha = 0
          }
        ) { _ in
          guard !self.isSearching else { return }
          self.dismissView.isHidden = true
          self.searchFieldView.isHidden = true
          self.iconsOverlay.resumeTracking()
          completion?()
        }
      }
    }

    func setConcealed(_ concealed: Bool, duration: TimeInterval) {
      guard concealed != isConcealed else { return }
      isConcealed = concealed
      isUserInteractionEnabled = !concealed
      accessibilityElementsHidden = concealed
      if concealed {
        iconsOverlay.pauseTracking()
      } else if !isSearching {
        iconsOverlay.resumeTracking()
      }
      let glasses: [(UIVisualEffectView, UIGlassEffect)] = [
        (tabGlassView, Self.interactiveGlass()), (createView, Self.interactiveGlass()),
      ]
      let contents: [UIView] = [tabContentView, searchButton]
      isHidden = false
      if duration == 0 {
        ContentBlur.clear(contents)
        UIView.performWithoutAnimation {
          for (glass, effect) in glasses { glass.effect = concealed ? nil : effect }
          for content in contents { content.alpha = concealed ? 0 : 1 }
          transform = .identity
        }
        isHidden = concealed
        return
      }
      let reduceMotion = UIAccessibility.isReduceMotionEnabled
      let pivot = bounds.height / 2 - Self.barHeight / 2
      let scale = CGAffineTransform(translationX: 0, y: pivot)
        .scaledBy(x: Self.concealScale, y: Self.concealScale)
        .translatedBy(x: 0, y: -pivot)
      if !reduceMotion {
        ContentBlur.animate(
          contents, to: concealed ? Self.concealBlurRadius : 0, duration: duration)
      }
      UIView.animate(
        springDuration: duration, bounce: 0,
        options: [.allowUserInteraction, .beginFromCurrentState],
        animations: {
          for (glass, effect) in glasses { glass.effect = concealed ? nil : effect }
          for content in contents { content.alpha = concealed ? 0 : 1 }
          self.transform = concealed && !reduceMotion ? scale : .identity
        }
      ) { finished in
        guard finished, self.isConcealed == concealed else { return }
        if concealed {
          self.isHidden = true
        } else {
          ContentBlur.clear(contents)
        }
      }
    }

    private var collapsedTabTransform: CGAffineTransform {
      let width = tabGlassView.bounds.width
      let scale = Self.collapsedTabScale
      return CGAffineTransform(translationX: -(width / 2) * (1 - scale), y: 0)
        .scaledBy(x: scale, y: 1)
    }

    static func barIcon(_ name: TIconName) -> UIImage {
      let image = UIImage(named: name.assetName, in: TDesignBundle.bundle, with: nil)!
      let size = CGSize(width: iconSide, height: iconSide)
      return UIGraphicsImageRenderer(size: size).image { _ in
        image.draw(in: CGRect(origin: .zero, size: size))
      }.withRenderingMode(.alwaysTemplate)
    }

    private static func interactiveGlass() -> UIGlassEffect {
      let effect = UIGlassEffect()
      effect.isInteractive = true
      return effect
    }

    private static func placeholder(for tab: MainTab) -> UIImage {
      let image = UIGraphicsImageRenderer(size: tab.image.size).image { _ in }
      image.accessibilityLabel = tab.label
      return image
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
    private let selectedIndexProvider: () -> Int

    init(
      images: [UIImage], normalTint: UIColor, accentTint: UIColor, iconSide: CGFloat,
      selectedIndexProvider: @escaping () -> Int
    ) {
      self.iconSide = iconSide
      self.selectedIndexProvider = selectedIndexProvider
      baseViews = images.map { image in
        let view = UIImageView(image: image.withRenderingMode(.alwaysTemplate))
        view.contentMode = .scaleAspectFit
        view.tintColor = normalTint
        return view
      }
      accentViews = images.map { image in
        let view = UIImageView(image: image.withRenderingMode(.alwaysTemplate))
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
      let index = selectedIndexProvider()
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
    private var originalIndex: Int?
    private weak var cachedLensView: UIView?
    var onLensMayMove: (() -> Void)?

    override var selectedSegmentIndex: Int {
      didSet { onLensMayMove?() }
    }

    init(placeholders: [UIImage]) {
      super.init(items: placeholders)
    }

    var lensView: UIView? {
      if let cachedLensView { return cachedLensView }
      let found = Self.findLens(in: self)
      cachedLensView = found
      return found
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
        selectedSegmentIndex = segmentIndex(at: touch.location(in: self))
      }
      super.touchesBegan(touches, with: event)
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
      onLensMayMove?()
      if let touch = touches.first {
        let index = segmentIndex(at: touch.location(in: self))
        if selectedSegmentIndex != index {
          selectedSegmentIndex = index
        }
      }
      super.touchesMoved(touches, with: event)
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
      onLensMayMove?()
      let startIndex = originalIndex
      originalIndex = nil
      super.touchesEnded(touches, with: event)
      guard startIndex != nil else { return }
      sendActions(for: .valueChanged)
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
      onLensMayMove?()
      if let originalIndex {
        selectedSegmentIndex = originalIndex
      }
      originalIndex = nil
      super.touchesCancelled(touches, with: event)
    }
  }

#endif
