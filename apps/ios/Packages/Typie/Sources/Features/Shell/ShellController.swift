#if canImport(UIKit)

  import Core
  import Design
  import FactoryKit
  import UIKit

  final class ShellController: UIViewController, UINavigationControllerDelegate,
    UIGestureRecognizerDelegate
  {
    private static let rootTransitionDuration: TimeInterval = 0.15
    private static let searchPushDuration: TimeInterval = 0.25
    private static let searchPopDuration: TimeInterval = 0.2
    private static let createButtonTintMix: Double = 0.07

    private let tabRoot: TabRootController
    private let openSearchHit: @MainActor (SearchHit, UIViewController) -> UIViewController?
    private let createItems: @MainActor (UIViewController) -> [CreateMenuItem]?
    private let session = SearchSession()
    private let navigation: ShellNavigationController
    private let menuDismissView = UIView()
    private var collapseMenu: (() -> Void)?
    private let selectionFeedback = UISelectionFeedbackGenerator()
    private var setBarSearching: ((Bool) -> Void)?
    private var setBarConcealed: ((Bool, TimeInterval) -> Void)?
    private var syncCreateButton: ((UIViewController?) -> Void)?
    private var applyCreateButton: ((TimeInterval?) -> Void)?
    private var createMenuAvailable = false
    private var barSearching = false
    private var barConcealed = false
    private var createFloating = false
    private var createButtonBottom: NSLayoutConstraint?
    private var barContentInset: CGFloat = 0

    init(
      tabRoot: TabRootController,
      openSearchHit: @escaping @MainActor (SearchHit, UIViewController) -> UIViewController?,
      createItems: @escaping @MainActor (UIViewController) -> [CreateMenuItem]?
    ) {
      self.tabRoot = tabRoot
      self.openSearchHit = openSearchHit
      self.createItems = createItems
      navigation = ShellNavigationController(root: tabRoot)
      super.init(nibName: nil, bundle: nil)
      navigation.delegate = self
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func viewDidLoad() {
      super.viewDidLoad()
      view.backgroundColor = .theme(\.surfaceCanvas)
      embed(navigation)
      navigation.interactivePopGestureRecognizer?.delegate = self
      guard #available(iOS 26, *) else { return }
      installCustomBar()
    }

    override func viewDidLayoutSubviews() {
      super.viewDidLayoutSubviews()
      guard #available(iOS 26, *) else { return }
      barContentInset = max(0, MainTabBar.contentBottomInset - view.safeAreaInsets.bottom)
      for controller in navigation.viewControllers {
        syncContentInset(of: controller)
      }
    }

    private func syncContentInset(of controller: UIViewController) {
      let inset = controller.hidesBottomBarWhenPushed ? 0 : barContentInset
      if controller.additionalSafeAreaInsets.bottom != inset {
        controller.additionalSafeAreaInsets.bottom = inset
      }
    }

    func navigationController(
      _ navigationController: UINavigationController, willShow viewController: UIViewController,
      animated: Bool
    ) {
      syncContentInset(of: viewController)
      let concealed = viewController.hidesBottomBarWhenPushed
      let interactive = navigationController.transitionCoordinator?.isInteractive == true
      if !interactive { syncCreateButton?(viewController) }
      if let coordinator = navigationController.transitionCoordinator {
        if coordinator.isInteractive {
          coordinator.notifyWhenInteractionChanges { [weak self] context in
            guard !context.isCancelled else { return }
            let remaining = context.transitionDuration * (1 - Double(context.percentComplete))
            self?.animateBar(concealed: concealed, duration: remaining)
          }
        } else {
          animateBar(concealed: concealed, duration: coordinator.transitionDuration)
        }
      } else {
        setBarConcealed?(concealed, 0)
      }
      guard !interactive else { return }
      syncSearchChrome(top: viewController)
    }

    func navigationController(
      _ navigationController: UINavigationController, didShow viewController: UIViewController,
      animated: Bool
    ) {
      setBarConcealed?(viewController.hidesBottomBarWhenPushed, 0)
      syncCreateButton?(viewController)
      syncSearchChrome(top: viewController)
    }

    private func animateBar(concealed: Bool, duration: TimeInterval) {
      setBarConcealed?(concealed, duration)
    }

    private func syncSearchChrome(top: UIViewController) {
      let searching = top is SearchScreenController
      if !searching { dismissKeyboard() }
      setBarSearching?(searching)
    }

    private func dismissKeyboard() {
      view.endEditing(true)
      session.releaseFocus()
    }

    func gestureRecognizerShouldBegin(_ gestureRecognizer: UIGestureRecognizer) -> Bool {
      navigation.viewControllers.count > 1
        && !(navigation.topViewController is SearchScreenController)
    }

    func navigationController(
      _ navigationController: UINavigationController,
      animationControllerFor operation: UINavigationController.Operation,
      from fromVC: UIViewController, to toVC: UIViewController
    ) -> (any UIViewControllerAnimatedTransitioning)? {
      if switchingTab { return FadeTransition(duration: Self.rootTransitionDuration) }
      guard operation == .pop, fromVC is SearchScreenController else { return nil }
      return FadeTransition(duration: Self.searchPopDuration)
    }

    private var switchingTab = false

    private func select(_ tab: MainTab) {
      let atRoot = navigation.viewControllers.count == 1
      guard tab != tabRoot.selectedTab || !atRoot else { return }
      let animated = !UIAccessibility.isReduceMotionEnabled
      guard atRoot else {
        tabRoot.select(tab, animated: false)
        switchingTab = true
        navigation.popToRootViewController(animated: animated)
        switchingTab = false
        return
      }
      tabRoot.select(tab, animated: animated)
      syncCreateButton?(tabRoot)
    }

    private func crossfade(duration: TimeInterval, _ change: @escaping () -> Void) {
      UIView.transition(
        with: navigation.view, duration: duration, options: .transitionCrossDissolve,
        animations: change)
    }

    private func openSearch() {
      guard !(navigation.topViewController is SearchScreenController) else { return }
      let screen = SearchScreenController(
        model: session.model, keyboardInset: barInset,
        onOpen: { [weak self] hit in self?.open(hit) },
        onDismissKeyboard: { [weak self] in self?.dismissKeyboard() })
      crossfade(duration: Self.searchPushDuration) {
        self.navigation.pushViewController(screen, animated: false)
      }
      session.requestFocus()
    }

    private var barInset: CGFloat {
      if #available(iOS 26, *) { MainTabBar.contentBottomInset } else { 0 }
    }

    private func closeSearch() {
      guard navigation.topViewController is SearchScreenController else { return }
      navigation.popViewController(animated: true)
    }

    private func open(_ hit: SearchHit) {
      guard let top = navigation.topViewController else { return }
      _ = openSearchHit(hit, top)
    }

    @available(iOS 26, *)
    private func installCustomBar() {
      let searchFieldHost = ThemedHostingController(
        title: "", SearchBarField(session: session))
      searchFieldHost.safeAreaRegions = []
      addChild(searchFieldHost)
      let progressHost = ThemedHostingController(
        title: "", SearchProgress(size: MainTabBar.iconSide))
      progressHost.safeAreaRegions = []
      addChild(progressHost)
      let bar = MainTabBar(
        selectedTint: .theme(\.textDefault), normalTint: .theme(\.textMuted),
        accentTint: .theme(\.paletteBlue), searchFieldView: searchFieldHost.view,
        progressView: progressHost.view)
      let backdrop = MainTabBarBackdrop(fadeColor: .theme(\.surfaceCanvas))
      let createState = CreateMenuState()
      let createMenuHost = ThemedHostingController(title: "", CreateMenu(state: createState))
      createMenuHost.safeAreaRegions = []
      addChild(createMenuHost)
      let createButton = CreateButtonView(
        tint: .theme { $0.surfaceInset.mix(with: $0.textDefault, by: Self.createButtonTintMix) },
        glyphTint: .theme(\.textDefault), state: createState, menuView: createMenuHost.view)
      createMenuHost.didMove(toParent: self)
      createButton.onHighlightChange = { [weak self] index in
        if index != nil { self?.selectionFeedback.selectionChanged() }
      }
      createButton.onCommit = { [weak createButton] index in
        guard let createButton, createState.items.indices.contains(index) else { return }
        createButton.setExpanded(false)
        createState.items[index].action()
      }
      searchFieldHost.didMove(toParent: self)
      progressHost.didMove(toParent: self)
      keepObserving(while: self) { [weak self, weak bar] in
        guard let self else { return }
        bar?.setSearchLoading(session.model.isSearching)
      }
      bar.onSelect = { [weak self] tab in
        self?.select(tab)
      }
      createButton.onExpandedChange = { [weak self] expanded in
        self?.menuDismissView.isHidden = !expanded
      }
      bar.searchButton.addAction(
        UIAction { [weak self] _ in self?.openSearch() }, for: .primaryActionTriggered)
      bar.dismissButton.addAction(
        UIAction { [weak self] _ in self?.closeSearch() }, for: .primaryActionTriggered)
      collapseMenu = { [weak createButton] in
        createButton?.setExpanded(false)
      }
      let chrome = Container.shared.bottomChrome()
      let raisedBottom = -(MainTabBar.barHeight + CreateButtonView.spacing)
      applyCreateButton = { [weak self, weak createButton] duration in
        guard let self else { return }
        let floating = barConcealed && createFloating
        let shown = createMenuAvailable && !barSearching && (!barConcealed || floating)
        createButton?.setShown(shown, duration: duration)
        chrome.set(inset: barConcealed && !floating ? 0 : MainTabBar.contentBottomInset)
        let bottom = floating ? 0 : raisedBottom
        guard createButtonBottom?.constant != bottom else { return }
        createButtonBottom?.constant = bottom
        let duration = duration ?? 0
        if duration > 0, !UIAccessibility.isReduceMotionEnabled {
          UIView.animate(springDuration: duration, bounce: 0) { self.view.layoutIfNeeded() }
        } else {
          view.layoutIfNeeded()
        }
      }
      setBarSearching = { [weak self, weak bar] searching in
        guard let self else { return }
        barSearching = searching
        bar?.setSearching(searching)
        applyCreateButton?(nil)
      }
      setBarConcealed = { [weak self, weak bar, weak backdrop] concealed, duration in
        guard let self else { return }
        barConcealed = concealed
        bar?.setConcealed(concealed, duration: duration)
        backdrop?.setConcealed(concealed, duration: duration)
        applyCreateButton?(duration)
      }
      syncCreateButton = { [weak self, weak createButton] top in
        guard let self else { return }
        let items = top.flatMap { createItems($0) }
        createButton?.setItems(items ?? [])
        createMenuAvailable = items != nil
        createFloating = top?.keepsCreateButtonWhenBarHidden ?? false
        applyCreateButton?(navigation.transitionCoordinator?.transitionDuration)
      }
      syncCreateButton?(navigation.topViewController)
      menuDismissView.isHidden = true
      menuDismissView.addGestureRecognizer(
        UITapGestureRecognizer(target: self, action: #selector(dismissMenu)))
      backdrop.translatesAutoresizingMaskIntoConstraints = false
      bar.translatesAutoresizingMaskIntoConstraints = false
      createButton.translatesAutoresizingMaskIntoConstraints = false
      menuDismissView.translatesAutoresizingMaskIntoConstraints = false
      view.addSubview(backdrop)
      view.addSubview(menuDismissView)
      view.addSubview(bar)
      view.addSubview(createButton)
      view.keyboardLayoutGuide.usesBottomSafeArea = false
      NSLayoutConstraint.activate([
        backdrop.leadingAnchor.constraint(equalTo: view.leadingAnchor),
        backdrop.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        backdrop.bottomAnchor.constraint(equalTo: view.keyboardLayoutGuide.topAnchor),
        backdrop.heightAnchor.constraint(equalToConstant: MainTabBarBackdrop.height),
        menuDismissView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
        menuDismissView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
        menuDismissView.topAnchor.constraint(equalTo: view.topAnchor),
        menuDismissView.bottomAnchor.constraint(equalTo: view.bottomAnchor),
        bar.leadingAnchor.constraint(
          equalTo: view.safeAreaLayoutGuide.leadingAnchor, constant: MainTabBar.horizontalPadding),
        bar.trailingAnchor.constraint(
          equalTo: view.safeAreaLayoutGuide.trailingAnchor, constant: -MainTabBar.horizontalPadding),
        bar.bottomAnchor.constraint(
          equalTo: view.keyboardLayoutGuide.topAnchor, constant: -MainTabBar.bottomPadding),
        createButton.trailingAnchor.constraint(equalTo: bar.trailingAnchor),
      ])
      let bottom = createButton.bottomAnchor.constraint(
        equalTo: bar.bottomAnchor, constant: raisedBottom)
      createButtonBottom = bottom
      bottom.isActive = true
      syncCreateButton?(navigation.topViewController)
    }

    @objc private func dismissMenu() {
      collapseMenu?()
    }
  }

#endif
