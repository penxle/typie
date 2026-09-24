#if canImport(UIKit)

  import Design
  internal import EditorFFI
  import UIKit

  @MainActor
  public final class EditorView: UIView {
    private static let pillTop: CGFloat = 12
    private static let pillTrailing: CGFloat = 16
    private static let guideThickness: CGFloat = 2

    public var debugOverlays: EditorDebugOverlays = [] {
      didSet {
        guard debugOverlays != oldValue else { return }
        withoutActions { showDebugOverlays() }
        if debugOverlays.contains(.pageSurfaces) != oldValue.contains(.pageSurfaces) {
          requestFrame()
        }
      }
    }

    public var isEditable = true {
      didSet {
        guard isEditable != oldValue else { return }
        withoutActions { configurePages(Array(pages.values)) }
      }
    }

    private let resources: EditorResources
    private let fonts: FontLoader
    private let scrollView = UIScrollView()
    private let contentView = UIView()
    private let scrollObserver = EditorScrollObserver()
    private let scrollbar = EditorScrollbar()
    private let pill = EditorPositionPill()
    private let indicator = PositionIndicatorModel()
    private let topGuide = UIView()
    private let bottomGuide = UIView()
    private let spacerArea = UIView()
    private let paddingArea = UIView()
    private let extensionArea = UIView()
    private var session: EditorSession?
    private var surface = EditorSurfaceModel()
    private var pages: [Int: EditorPageView] = [:]
    private var geometry: FrameGeometry?
    private var motion = ScrollMotion()
    private var displayLink: CADisplayLink?
    private var frameNeeded = false
    private var applying = false
    private var scrubbing = false
    private var barScrolling = false
    private var fastPace = false
    private var failed = false
    private var openings = 0
    private var sizeWaiters: [CheckedContinuation<Void, Never>] = []

    public init(resources: EditorResources, fonts: FontLoader) {
      self.resources = resources
      self.fonts = fonts
      super.init(frame: .zero)
      scrollView.alwaysBounceVertical = true
      scrollView.showsVerticalScrollIndicator = false
      scrollView.backgroundColor = .theme(\.surfaceCanvas)
      scrollView.delegate = scrollObserver
      scrollObserver.onScroll = { [weak self] in self?.scrolled() }
      scrollObserver.onInsetChange = { [weak self] in
        self?.layoutGuides()
        self?.layoutScrollbar()
        self?.requestFrame()
      }
      scrollObserver.onDragStart = { [weak self] in
        self?.moved { $0.willBeginDragging() }
      }
      scrollObserver.onDragRelease = { [weak self] target in
        self?.moved { $0.willEndDragging(target: target) }
      }
      scrollObserver.onDragEnd = { [weak self] willDecelerate in
        self?.moved { $0.didEndDragging(willDecelerate: willDecelerate) }
      }
      scrollObserver.onDecelerationEnd = { [weak self] in
        self?.moved { $0.didEndDecelerating() }
      }
      scrollObserver.shouldScrollToTop = { [weak self] top, current in
        guard let self else { return true }
        guard !scrubbing else { return false }
        let pixel = 1 / traitCollection.displayScale
        moved { $0.willScrollToTop(top: top, current: current, pixel: pixel) }
        return true
      }
      scrollObserver.onScrollToTopEnd = { [weak self] in
        self?.moved { $0.didScrollToTop() }
      }
      addSubview(scrollView)
      scrollView.addSubview(contentView)
      for (area, color) in [
        (spacerArea, EditorDebugColors.topSpacer), (paddingArea, EditorDebugColors.bottomPadding),
        (extensionArea, EditorDebugColors.extensionFill),
      ] {
        area.backgroundColor = color
        area.isUserInteractionEnabled = false
        area.isHidden = true
        contentView.addSubview(area)
      }
      for guide in [topGuide, bottomGuide] {
        guide.backgroundColor = EditorDebugColors.viewportGuide
        guide.isUserInteractionEnabled = false
        guide.isHidden = true
        addSubview(guide)
      }
      addSubview(scrollbar)
      scrollbar.install(on: self, over: scrollView)
      scrollbar.onGrab = { [weak self] grabbed in self?.grabbed(grabbed) }
      scrollbar.onScroll = { [weak self] position in self?.scroll(to: position) }
      pill.translatesAutoresizingMaskIntoConstraints = false
      addSubview(pill)
      NSLayoutConstraint.activate([
        pill.topAnchor.constraint(equalTo: safeAreaLayoutGuide.topAnchor, constant: Self.pillTop),
        pill.trailingAnchor.constraint(
          equalTo: safeAreaLayoutGuide.trailingAnchor, constant: -Self.pillTrailing),
      ])
      indicator.onChange = { [weak self] in self?.showIndicator() }
      registerForTraitChanges([UITraitUserInterfaceStyle.self, UITraitDisplayScale.self]) {
        (self: EditorView, _) in
        self.traitsChanged()
      }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    public func open(_ document: EditorDocument) async throws {
      openings += 1
      let opening = openings
      if window == nil {
        layoutIfNeeded()
      }
      while scrollView.bounds.width <= 0 || scrollView.bounds.height <= 0 {
        await withCheckedContinuation { sizeWaiters.append($0) }
      }
      guard opening == openings else { return }
      let session: EditorSession
      do {
        try resources.setTheme(isDark: traitCollection.userInterfaceStyle == .dark)
        let request = viewportRequest(time: CACurrentMediaTime())
        session = try await EditorSession.open(
          document, resources: resources, fonts: fonts,
          viewport: Viewport(
            width: Float(request.width), height: Float(request.height),
            scaleFactor: request.deviceScale))
      } catch {
        guard opening == openings else { return }
        throw error
      }
      guard opening == openings else { return }
      reset()
      session.onNeedsFrame = { [weak self] in self?.needsFrame() }
      self.session = session
      do {
        try produceFrame(time: CACurrentMediaTime())
      } catch {
        fail()
        throw error
      }
    }

    public override func layoutSubviews() {
      super.layoutSubviews()
      let resized = scrollView.frame != bounds
      scrollView.frame = bounds
      layoutGuides()
      layoutScrollbar()
      if scrollView.bounds.width > 0, scrollView.bounds.height > 0, !sizeWaiters.isEmpty {
        let waiters = sizeWaiters
        sizeWaiters = []
        for waiter in waiters {
          waiter.resume()
        }
      }
      if resized {
        requestFrame()
      }
    }

    public override func didMoveToWindow() {
      super.didMoveToWindow()
      if window != nil {
        startPump()
      } else {
        scrollbar.cancelGrab()
        stopPump()
        if motion.reset() {
          needsFrame()
        }
      }
    }

    private var visibleHeight: Double {
      let insets = scrollView.adjustedContentInset
      return max(0, scrollView.bounds.height - insets.top - insets.bottom)
    }

    private func viewportRequest(time: CFTimeInterval) -> ViewportRequest {
      let insets = scrollView.adjustedContentInset
      return EditorViewGeometry.request(
        size: scrollView.bounds.size, offset: scrollView.contentOffset,
        insets: EditorInsets(
          top: insets.top, left: insets.left, bottom: insets.bottom, right: insets.right),
        deviceScale: traitCollection.displayScale, timeMs: time * 1000,
        debug: debugOverlays.contains(.pageSurfaces), destination: motion.destination)
    }

    private func produceFrame(time: CFTimeInterval) throws {
      guard let session else { return }
      frameNeeded = false
      apply(try session.frame(viewportRequest(time: time)))
    }

    private func requestFrame(time: CFTimeInterval = CACurrentMediaTime()) {
      guard !applying else {
        needsFrame()
        return
      }
      guard session != nil, !failed else { return }
      do {
        try produceFrame(time: time)
      } catch {
        fail()
      }
    }

    private func needsFrame() {
      frameNeeded = true
      displayLink?.isPaused = false
    }

    private func fail() {
      failed = true
      displayLink?.isPaused = true
    }

    private func reset() {
      session = nil
      let insets = scrollView.adjustedContentInset
      scrollView.contentOffset = CGPoint(x: -insets.left, y: -insets.top)
      for page in pages.values {
        page.removeFromSuperview()
      }
      pages = [:]
      surface = EditorSurfaceModel()
      geometry = nil
      failed = false
      frameNeeded = false
      _ = motion.reset()
      indicator.autoScrolled(indicatorText())
      scrollbar.hide()
    }

    private func apply(_ update: EditorFrameUpdate) {
      let previous = geometry
      let changes = surface.apply(update)
      geometry = update.geometry
      applying = true
      CATransaction.begin()
      CATransaction.setDisableActions(true)
      if scrollView.contentSize != changes.contentSize {
        scrollView.contentSize = changes.contentSize
      }
      contentView.frame = CGRect(origin: .zero, size: changes.contentSize)
      for index in changes.removedPages {
        pages.removeValue(forKey: index)?.removeFromSuperview()
      }
      var added: [EditorPageView] = []
      for page in changes.addedPages {
        let view = EditorPageView(frame: page.frame)
        contentView.addSubview(view)
        pages[page.index] = view
        added.append(view)
      }
      for page in changes.movedPages {
        pages[page.index]?.frame = page.frame
      }
      for key in changes.droppedTiles {
        pages[key.page]?.dropTile(key.bounds)
      }
      for tile in changes.setTiles {
        pages[tile.key.page]?.setTile(tile)
      }
      if previous?.layout != update.geometry.layout || previous?.zoom != update.geometry.zoom {
        configurePages(Array(pages.values))
        var canvas = UIColor.theme(\.surfaceDefault)
        if case .paginated = update.geometry.layout {
          canvas = .theme(\.surfaceCanvas)
        }
        scrollView.backgroundColor = canvas
      } else {
        configurePages(added)
      }
      showDebugOverlays()
      CATransaction.commit()
      applying = false
      indicator.positionChanged(indicatorText())
      layoutScrollbar()
      do {
        try session?.presented(update.geometry.id)
      } catch {
        fail()
        return
      }
      if update.geometry.fillRemaining || update.geometry.needsNextFrame {
        displayLink?.isPaused = false
      }
    }

    private func configurePages(_ views: [EditorPageView]) {
      guard let geometry else { return }
      for view in views {
        view.configure(
          layout: geometry.layout, zoom: geometry.zoom, isEditable: isEditable,
          traits: traitCollection)
      }
    }

    private func moved(_ event: (inout ScrollMotion) -> Bool) {
      guard event(&motion) else { return }
      if motion.destination != nil {
        displayLink?.isPaused = false
      }
      requestFrame()
    }

    private func scrolled() {
      layoutScrollbar()
      guard !applying else {
        needsFrame()
        return
      }
      let user =
        scrubbing || barScrolling || scrollView.isTracking || scrollView.isDragging
        || scrollView.isDecelerating
      if user {
        indicator.userScrolled(indicatorText())
      } else {
        indicator.autoScrolled(indicatorText())
      }
      scrollbar.scrolled(automatic: !user)
      requestFrame()
    }

    private func setScrubbing(_ isScrubbing: Bool) {
      scrubbing = isScrubbing
      indicator.scrubbingChanged(isScrubbing)
    }

    private func grabbed(_ isGrabbed: Bool) {
      setScrubbing(isGrabbed)
      guard isGrabbed else { return }
      let cleared = motion.reset()
      let shown = geometry?.id
      let layout = scrollbar.layout
      let position = min(max(layout.scrollPosition, 0), layout.maxScroll)
      scrollView.setContentOffset(
        CGPoint(
          x: scrollView.contentOffset.x, y: position - scrollView.adjustedContentInset.top),
        animated: false)
      if cleared, geometry?.id == shown {
        requestFrame()
      }
    }

    private func scroll(to position: Double) {
      barScrolling = true
      scrollView.contentOffset.y = position - scrollView.adjustedContentInset.top
      barScrolling = false
    }

    private func layoutScrollbar() {
      let insets = scrollView.adjustedContentInset
      let height = visibleHeight
      scrollbar.frame = CGRect(
        x: bounds.width - safeAreaInsets.right - EditorScrollbar.laneWidth, y: insets.top,
        width: EditorScrollbar.laneWidth, height: height)
      scrollbar.layout = EditorScrollbarLayout(
        visibleHeight: height, contentHeight: scrollView.contentSize.height,
        scrollPosition: scrollView.contentOffset.y + insets.top)
      scrollbar.accessibilityValue = indicatorText()
    }

    private func indicatorText() -> String? {
      guard let geometry, geometry.contentHeight > visibleHeight else { return nil }
      return PositionIndicatorModel.text(position: geometry.position, layout: geometry.layout)
    }

    private func showIndicator() {
      let text = indicator.text
      let instant = text == nil || pill.text == nil
      pill.text = text
      pill.setShown(
        indicator.isVisible, duration: instant ? 0 : PositionIndicatorModel.fadeDuration)
    }

    private func traitsChanged() {
      try? resources.setTheme(isDark: traitCollection.userInterfaceStyle == .dark)
      withoutActions {
        for view in pages.values {
          view.resolveColors(traitCollection)
        }
      }
      requestFrame()
    }

    private func withoutActions(_ body: () -> Void) {
      CATransaction.begin()
      CATransaction.setDisableActions(true)
      body()
      CATransaction.commit()
    }

    private func layoutGuides() {
      let insets = scrollView.adjustedContentInset
      topGuide.frame = CGRect(
        x: 0, y: insets.top, width: bounds.width, height: Self.guideThickness)
      bottomGuide.frame = CGRect(
        x: 0, y: bounds.height - insets.bottom, width: bounds.width, height: Self.guideThickness)
    }

    private func showDebugOverlays() {
      let guides = debugOverlays.contains(.viewportGuides)
      topGuide.isHidden = !guides
      bottomGuide.isHidden = !guides
      let areas = debugOverlays.contains(.bodyAreas) && geometry != nil
      for area in [spacerArea, paddingArea, extensionArea] {
        area.isHidden = !areas
      }
      if areas, let geometry {
        let body = EditorViewGeometry.bodyAreas(geometry)
        spacerArea.frame = body.spacer
        paddingArea.frame = body.padding
        extensionArea.frame = body.fill
      }
      let surfaces = debugOverlays.contains(.pageSurfaces)
      for (index, view) in pages {
        view.showSurfaceDebug(
          surfaces
            ? geometry.map {
              EditorViewGeometry.surfaceDebug(page: index, size: view.bounds.size, geometry: $0)
            } : nil)
      }
    }

    private func startPump() {
      guard displayLink == nil else { return }
      let link = CADisplayLink(target: self, selector: #selector(step(_:)))
      link.isPaused =
        !(frameNeeded || geometry?.fillRemaining == true || geometry?.needsNextFrame == true)
      link.add(to: .main, forMode: .common)
      displayLink = link
    }

    private func stopPump() {
      displayLink?.invalidate()
      displayLink = nil
      fastPace = false
    }

    private func pausePump() {
      displayLink?.isPaused = true
      if fastPace {
        fastPace = false
        displayLink?.preferredFrameRateRange = .default
      }
    }

    private func pace(_ link: CADisplayLink) {
      let fast = geometry?.fillRemaining == true || motion.destination != nil
      guard fast != fastPace else { return }
      fastPace = fast
      if fast, let rate = window?.windowScene?.screen.maximumFramesPerSecond {
        let top = Float(rate)
        link.preferredFrameRateRange = CAFrameRateRange(minimum: top, maximum: top, preferred: top)
      } else {
        link.preferredFrameRateRange = .default
      }
    }

    @objc private func step(_ link: CADisplayLink) {
      guard let session, !failed else {
        pausePump()
        return
      }
      if motion.validate(
        isDecelerating: scrollView.isDecelerating, isTracking: scrollView.isTracking)
      {
        frameNeeded = true
      }
      pace(link)
      if frameNeeded || geometry?.needsNextFrame == true {
        requestFrame(time: link.timestamp)
        return
      }
      guard geometry?.fillRemaining == true else {
        if motion.destination == nil {
          pausePump()
        }
        return
      }
      do {
        if let update = try session.fill(budgetMs: FillPolicy.budgetMs) {
          apply(update)
        } else {
          geometry?.fillRemaining = false
          if motion.destination == nil {
            pausePump()
          }
        }
      } catch {
        fail()
      }
    }
  }

  @MainActor
  private final class EditorScrollObserver: NSObject, UIScrollViewDelegate {
    var onScroll: (() -> Void)?
    var onInsetChange: (() -> Void)?
    var onDragStart: (() -> Void)?
    var onDragRelease: ((Double) -> Void)?
    var onDragEnd: ((Bool) -> Void)?
    var onDecelerationEnd: (() -> Void)?
    var shouldScrollToTop: ((Double, Double) -> Bool)?
    var onScrollToTopEnd: (() -> Void)?

    func scrollViewDidScroll(_ scrollView: UIScrollView) {
      onScroll?()
    }

    func scrollViewDidChangeAdjustedContentInset(_ scrollView: UIScrollView) {
      onInsetChange?()
    }

    func scrollViewWillBeginDragging(_ scrollView: UIScrollView) {
      onDragStart?()
    }

    func scrollViewWillEndDragging(
      _ scrollView: UIScrollView, withVelocity velocity: CGPoint,
      targetContentOffset: UnsafeMutablePointer<CGPoint>
    ) {
      onDragRelease?(targetContentOffset.pointee.y)
    }

    func scrollViewDidEndDragging(_ scrollView: UIScrollView, willDecelerate decelerate: Bool) {
      onDragEnd?(decelerate)
    }

    func scrollViewDidEndDecelerating(_ scrollView: UIScrollView) {
      onDecelerationEnd?()
    }

    func scrollViewShouldScrollToTop(_ scrollView: UIScrollView) -> Bool {
      shouldScrollToTop?(-scrollView.adjustedContentInset.top, scrollView.contentOffset.y) ?? true
    }

    func scrollViewDidScrollToTop(_ scrollView: UIScrollView) {
      onScrollToTopEnd?()
    }
  }

#endif
