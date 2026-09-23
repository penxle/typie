#if canImport(UIKit)

  import Design
  import UIKit

  final class TabRootController: UIViewController {
    private static let transitionDuration: TimeInterval = 0.15

    private let provider: @MainActor (MainTab) -> UIViewController
    private var tabs: [MainTab: UIViewController] = [:]
    private var titleLabels: [MainTab: UILabel] = [:]
    private let titleHost = TitleHostView()
    private(set) var selectedTab: MainTab
    private(set) var current: UIViewController

    var sharedRightItems: [UIBarButtonItem] = [] {
      didSet { mirror(animated: false) }
    }

    init(initial: MainTab, provider: @escaping @MainActor (MainTab) -> UIViewController) {
      self.provider = provider
      selectedTab = initial
      current = provider(initial)
      super.init(nibName: nil, bundle: nil)
      tabs[initial] = current
      navigationItem.backButtonDisplayMode = .minimal
      for tab in MainTab.allCases {
        let label = UILabel()
        label.text = tab.label
        label.textColor = .theme(\.textDefault)
        titleLabels[tab] = label
      }
      layoutTitleLabels()
      navigationItem.titleView = titleHost
      mirror(animated: false)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func viewDidLoad() {
      super.viewDidLoad()
      view.backgroundColor = .clear
      embed(current)
      registerForTraitChanges([UITraitPreferredContentSizeCategory.self]) {
        (self: TabRootController, _) in
        self.layoutTitleLabels()
        self.navigationController?.navigationBar.setNeedsLayout()
      }
      mirror(animated: false)
    }

    func select(_ tab: MainTab, animated: Bool) {
      guard tab != selectedTab else { return }
      let from = current
      let to = tabs[tab] ?? provider(tab)
      tabs[tab] = to
      selectedTab = tab
      current = to
      mirror(animated: animated)
      guard isViewLoaded else { return }
      from.willMove(toParent: nil)
      addChild(to)
      to.view.frame = view.bounds
      to.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
      to.view.alpha = 0
      view.addSubview(to.view)
      to.view.layoutIfNeeded()
      let finish = {
        from.view.removeFromSuperview()
        from.removeFromParent()
        to.didMove(toParent: self)
      }
      guard animated else {
        to.view.alpha = 1
        finish()
        return
      }
      let animator = UIViewPropertyAnimator(duration: Self.transitionDuration, curve: .easeOut) {
        to.view.alpha = 1
      }
      animator.addCompletion { _ in finish() }
      animator.startAnimation()
    }

    private func mirror(animated: Bool) {
      if isViewLoaded { current.loadViewIfNeeded() }
      let item = current.navigationItem
      titleHost.show(item.titleView ?? titleLabels[selectedTab]!, animated: animated)
      navigationItem.setRightBarButtonItems(
        sharedRightItems + (item.rightBarButtonItems ?? []), animated: animated)
      creationContext = current.creationContext
    }

    private func layoutTitleLabels() {
      var size = CGSize.zero
      for label in titleLabels.values {
        label.font = TTypography.title.uiFont(for: traitCollection)
        label.sizeToFit()
        size.width = max(size.width, label.bounds.width)
        size.height = max(size.height, label.bounds.height)
      }
      titleHost.fixedSize = size
    }
  }

  private final class TitleHostView: UIView {
    private static let transitionDuration: TimeInterval = 0.15

    private(set) var currentView: UIView?

    var fixedSize = CGSize.zero {
      didSet {
        frame = CGRect(origin: frame.origin, size: fixedSize)
        invalidateIntrinsicContentSize()
        setNeedsLayout()
      }
    }

    override func sizeThatFits(_ size: CGSize) -> CGSize {
      fixedSize
    }

    override var intrinsicContentSize: CGSize {
      fixedSize
    }

    override func layoutSubviews() {
      super.layoutSubviews()
      for subview in subviews {
        subview.center = CGPoint(x: bounds.midX, y: bounds.midY)
      }
    }

    func show(_ view: UIView, animated: Bool) {
      guard view !== currentView else { return }
      let previous = currentView
      currentView = view
      view.alpha = 0
      addSubview(view)
      setNeedsLayout()
      layoutIfNeeded()
      let change = {
        view.alpha = 1
        previous?.alpha = 0
      }
      let finish = { [weak self] in
        guard let previous, previous !== self?.currentView else { return }
        previous.removeFromSuperview()
        previous.alpha = 1
      }
      guard animated else {
        change()
        finish()
        return
      }
      UIView.animate(
        withDuration: Self.transitionDuration, delay: 0,
        options: [.curveEaseOut, .beginFromCurrentState], animations: change
      ) { _ in finish() }
    }
  }

#endif
