#if canImport(UIKit)

  import Core
  import Design
  import UIKit

  final class HomeHostController: UIViewController {
    private let search: HomeSearchState
    private let content: UIViewController
    private var barCollapsed = false
    private var detachedLeftItem: UIBarButtonItem?

    init(search: HomeSearchState, content: UIViewController) {
      self.search = search
      self.content = content
      super.init(nibName: nil, bundle: nil)
      navigationItem.backButtonDisplayMode = .minimal
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    private let titleLabel = UILabel()

    override func viewDidLoad() {
      super.viewDidLoad()
      view.backgroundColor = .clear
      embed(content)
      titleLabel.text = title
      navigationItem.title = nil
      titleLabel.font = TTypography.title.uiFont(for: traitCollection)
      titleLabel.adjustsFontForContentSizeCategory = true
      titleLabel.textColor = .theme(\.textDefault)
      titleLabel.alpha = 0
      titleLabel.sizeToFit()
      let titleContainer = UIView(frame: titleLabel.bounds)
      titleContainer.addSubview(titleLabel)
      navigationItem.titleView = titleContainer
      registerForTraitChanges([UITraitPreferredContentSizeCategory.self]) {
        (self: HomeHostController, _) in
        self.layoutTitle()
      }
      keepObserving(while: self) { [weak self] in
        guard let self else { return }
        let visible = search.titleVisible
        let alpha: CGFloat = visible ? 1 : 0
        guard titleLabel.alpha != alpha else { return }
        let duration = UIAccessibility.isReduceMotionEnabled ? 0 : 0.2
        UIView.animate(withDuration: duration) { self.titleLabel.alpha = alpha }
      }
      keepObserving(while: self) { [weak self] in
        guard let self else { return }
        let active = search.isActive
        guard view.window != nil, let navigation = navigationController, barCollapsed != active
        else { return }
        setBarCollapsed(active, in: navigation)
      }
    }

    private func layoutTitle() {
      titleLabel.font = TTypography.title.uiFont(for: traitCollection)
      titleLabel.sizeToFit()
      navigationItem.titleView?.frame = titleLabel.bounds
      navigationController?.navigationBar.setNeedsLayout()
    }

    override func viewWillAppear(_ animated: Bool) {
      super.viewWillAppear(animated)
      guard let navigation = navigationController else { return }
      barCollapsed = search.isActive
      syncBarInset()
      let bar = navigation.navigationBar
      guard barCollapsed else {
        bar.transform = .identity
        bar.alpha = 1
        return
      }
      guard let coordinator = transitionCoordinator else {
        bar.transform = collapsedTransform(in: navigation)
        return
      }
      bar.transform = .identity
      coordinator.animate(alongsideTransition: { _ in bar.alpha = 0 }) { context in
        if context.isCancelled {
          bar.alpha = 1
        } else {
          bar.transform = self.collapsedTransform(in: navigation)
          bar.alpha = 1
        }
      }
    }

    func prepareForPush() {
      guard barCollapsed, let leftItem = navigationItem.leftBarButtonItem else { return }
      detachedLeftItem = leftItem
      navigationItem.leftBarButtonItem = nil
    }

    override func viewWillDisappear(_ animated: Bool) {
      super.viewWillDisappear(animated)
      view.endEditing(true)
      guard barCollapsed, let navigation = navigationController else { return }
      let bar = navigation.navigationBar
      bar.transform = .identity
      guard let coordinator = transitionCoordinator, !coordinator.isInteractive,
        !coordinator.isCancelled
      else {
        bar.alpha = 1
        return
      }
      bar.alpha = 0
      let scheduled = coordinator.animate(alongsideTransition: { _ in bar.alpha = 1 }) { _ in
        bar.alpha = 1
      }
      if !scheduled {
        bar.alpha = 1
      }
    }

    override func viewDidLayoutSubviews() {
      super.viewDidLayoutSubviews()
      syncBarInset()
    }

    private func syncBarInset() {
      guard let navigation = navigationController else { return }
      let height = navigation.navigationBar.frame.height
      if height > 0, search.barHeight != height {
        search.barHeight = height
      }
      let inset = -height
      if additionalSafeAreaInsets.top != inset {
        additionalSafeAreaInsets.top = inset
      }
    }

    private func collapsedTransform(in navigation: UINavigationController) -> CGAffineTransform {
      CGAffineTransform(
        translationX: 0, y: -(navigation.view.safeAreaInsets.top + search.barHeight))
    }

    private func setBarCollapsed(_ collapsed: Bool, in navigation: UINavigationController) {
      barCollapsed = collapsed
      if !collapsed, let detachedLeftItem {
        navigationItem.leftBarButtonItem = detachedLeftItem
        self.detachedLeftItem = nil
      }
      let bar = navigation.navigationBar
      let duration = UIAccessibility.isReduceMotionEnabled ? 0 : HomeSearchState.transitionDuration
      UIView.animate(
        springDuration: duration, bounce: HomeSearchState.transitionBounce,
        options: [.allowUserInteraction]
      ) {
        bar.transform = collapsed ? self.collapsedTransform(in: navigation) : .identity
      } completion: { _ in
        guard collapsed, self.barCollapsed, let leftItem = self.navigationItem.leftBarButtonItem
        else { return }
        self.detachedLeftItem = leftItem
        self.navigationItem.leftBarButtonItem = nil
      }
    }
  }

#endif
