#if canImport(UIKit)

  import Core
  import Design
  import UIKit

  final class ShellController: UIViewController, UINavigationControllerDelegate,
    UIGestureRecognizerDelegate
  {
    private static let rootTransitionDuration: TimeInterval = 0.15
    private static let searchPushDuration: TimeInterval = 0.25
    private static let searchPopDuration: TimeInterval = 0.2

    private let rootProvider: @MainActor (MainTab) -> UIViewController
    private let openSearchHit: @MainActor (SearchHit, UIViewController) -> UIViewController?
    private let createMenu: @MainActor (UIViewController) -> UIMenu?
    private let session = SearchSession()
    private let navigation: ShellNavigationController
    private var roots: [MainTab: UIViewController] = [:]
    private var selectedTab = MainTab.initial
    private let menuDismissView = UIView()
    private var collapseMenu: (() -> Void)?
    private var moreMenuState: MoreMenuState?
    private let selectionFeedback = UISelectionFeedbackGenerator()
    private var setBarSearching: ((Bool) -> Void)?
    private var setBarConcealed: ((Bool) -> Void)?
    private var syncCreateButton: ((UIViewController?) -> Void)?
    private var barContentInset: CGFloat = 0

    init(
      rootProvider: @escaping @MainActor (MainTab) -> UIViewController,
      openSearchHit: @escaping @MainActor (SearchHit, UIViewController) -> UIViewController?,
      createMenu: @escaping @MainActor (UIViewController) -> UIMenu?
    ) {
      self.rootProvider = rootProvider
      self.openSearchHit = openSearchHit
      self.createMenu = createMenu
      let root = rootProvider(MainTab.initial)
      navigation = ShellNavigationController(root: root)
      super.init(nibName: nil, bundle: nil)
      roots[MainTab.initial] = root
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
        setBarConcealed?(concealed)
      }
      guard navigationController.transitionCoordinator?.isInteractive != true else { return }
      syncCreateButton?(viewController)
      syncSearchChrome(top: viewController)
    }

    func navigationController(
      _ navigationController: UINavigationController, didShow viewController: UIViewController,
      animated: Bool
    ) {
      setBarConcealed?(viewController.hidesBottomBarWhenPushed)
      syncCreateButton?(viewController)
      syncSearchChrome(top: viewController)
    }

    private func animateBar(concealed: Bool, duration: TimeInterval) {
      UIView.animate(
        withDuration: duration, delay: 0, options: [.allowUserInteraction, .beginFromCurrentState]
      ) { [weak self] in
        self?.setBarConcealed?(concealed)
      }
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

    private func root(for tab: MainTab) -> UIViewController {
      if let root = roots[tab] { return root }
      let root = rootProvider(tab)
      roots[tab] = root
      return root
    }

    private func select(_ tab: MainTab) {
      guard tab != selectedTab || navigation.viewControllers.count > 1 else { return }
      selectedTab = tab
      let root = root(for: tab)
      switchingTab = true
      navigation.setViewControllers([root], animated: !UIAccessibility.isReduceMotionEnabled)
      switchingTab = false
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
      let menuState = MoreMenuState(
        items: [
          MoreMenuItem(icon: LucideIcon.settings, title: "스페이스 설정"),
          MoreMenuItem(icon: LucideIcon.externalLink, title: "스페이스 열기"),
          MoreMenuItem(icon: LucideIcon.trash2, title: "휴지통"),
          MoreMenuItem(icon: LucideIcon.squarePlus, title: "새 스페이스 생성"),
          MoreMenuItem(icon: LucideIcon.sunMoon, title: "테마"),
          MoreMenuItem(icon: LucideIcon.userRound, title: "프로필"),
          MoreMenuItem(icon: LucideIcon.settings, title: "설정"),
          MoreMenuItem(icon: LucideIcon.ellipsis, title: "더 보기"),
        ], profileName: "Finn")
      moreMenuState = menuState
      let menuHost = ThemedHostingController(title: "", MoreMenu(state: menuState))
      menuHost.safeAreaRegions = []
      addChild(menuHost)
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
        accentTint: .theme(\.paletteBlue), menuView: menuHost.view,
        menuHeaderHeight: MoreMenu.headerHeight, menuRowHeight: MoreMenu.rowHeight,
        menuRowCount: menuState.items.count, searchFieldView: searchFieldHost.view,
        progressView: progressHost.view, createTint: .theme(\.accentDefault),
        onCreateTint: .theme(\.surfaceCanvas))
      menuHost.didMove(toParent: self)
      searchFieldHost.didMove(toParent: self)
      progressHost.didMove(toParent: self)
      keepObserving(while: self) { [weak self, weak bar] in
        guard let self else { return }
        bar?.setSearchLoading(session.model.isSearching)
      }
      bar.onScrubHighlight = { [weak self] index in
        guard let self, let moreMenuState, moreMenuState.highlightedIndex != index else { return }
        moreMenuState.highlightedIndex = index
        if index != nil { selectionFeedback.selectionChanged() }
      }
      bar.onScrubCommit = { [weak self] in
        guard let self, let moreMenuState else { return }
        if moreMenuState.highlightedIndex != nil {
          moreMenuState.highlightedIndex = nil
          collapseMenu?()
        }
      }
      bar.onSelect = { [weak self] tab in
        self?.select(tab)
      }
      bar.onExpandedChange = { [weak self] expanded in
        self?.menuDismissView.isHidden = !expanded
        self?.moreMenuState?.isPresented = expanded
        if !expanded { self?.moreMenuState?.highlightedIndex = nil }
      }
      bar.searchButton.addAction(
        UIAction { [weak self] _ in self?.openSearch() }, for: .primaryActionTriggered)
      bar.dismissButton.addAction(
        UIAction { [weak self] _ in self?.closeSearch() }, for: .primaryActionTriggered)
      collapseMenu = { [weak bar] in bar?.setExpanded(false) }
      setBarSearching = { [weak bar] searching in bar?.setSearching(searching) }
      setBarConcealed = { [weak bar] concealed in bar?.setConcealed(concealed) }
      syncCreateButton = { [weak self, weak bar] top in
        guard let self, let bar else { return }
        let menu = top.flatMap { createMenu($0) }
        bar.createButton.menu = menu
        bar.setCreateButtonVisible(menu != nil)
      }
      syncCreateButton?(navigation.topViewController)
      menuDismissView.isHidden = true
      menuDismissView.addGestureRecognizer(
        UITapGestureRecognizer(target: self, action: #selector(dismissMenu)))
      bar.translatesAutoresizingMaskIntoConstraints = false
      menuDismissView.translatesAutoresizingMaskIntoConstraints = false
      view.addSubview(menuDismissView)
      view.addSubview(bar)
      view.keyboardLayoutGuide.usesBottomSafeArea = false
      NSLayoutConstraint.activate([
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
      ])
    }

    @objc private func dismissMenu() {
      collapseMenu?()
    }
  }

#endif
