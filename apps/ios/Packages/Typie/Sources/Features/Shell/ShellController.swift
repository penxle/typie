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
    private static let createButtonTintMix: Double = 0.1

    private let rootProvider: @MainActor (MainTab) -> UIViewController
    private let openSearchHit: @MainActor (SearchHit, UIViewController) -> UIViewController?
    private let createItems: @MainActor (UIViewController) -> [CreateMenuItem]?
    private let moreMenuItems: [MoreMenuItem]
    private let session = SearchSession()
    private let navigation: ShellNavigationController
    private var roots: [MainTab: UIViewController] = [:]
    private var selectedTab = MainTab.initial
    private let menuDismissView = UIView()
    private var collapseMenu: (() -> Void)?
    private var moreMenuState: MoreMenuState?
    private let selectionFeedback = UISelectionFeedbackGenerator()
    private var setBarSearching: ((Bool) -> Void)?
    private var setBarConcealed: ((Bool, TimeInterval) -> Void)?
    private var syncCreateButton: ((UIViewController?) -> Void)?
    private var applyCreateButton: ((TimeInterval?) -> Void)?
    private var createMenuAvailable = false
    private var barSearching = false
    private var barConcealed = false
    private var barContentInset: CGFloat = 0

    init(
      rootProvider: @escaping @MainActor (MainTab) -> UIViewController,
      openSearchHit: @escaping @MainActor (SearchHit, UIViewController) -> UIViewController?,
      createItems: @escaping @MainActor (UIViewController) -> [CreateMenuItem]?,
      moreMenuItems: [MoreMenuItem]
    ) {
      self.rootProvider = rootProvider
      self.openSearchHit = openSearchHit
      self.createItems = createItems
      self.moreMenuItems = moreMenuItems
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
      let interactive = navigationController.transitionCoordinator?.isInteractive == true
      if !interactive, !concealed { syncCreateButton?(viewController) }
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
      if concealed { syncCreateButton?(viewController) }
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
        ] + moreMenuItems, profileName: "Finn")
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
        progressView: progressHost.view)
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
        guard let self, let moreMenuState, let index = moreMenuState.highlightedIndex else {
          return
        }
        moreMenuState.highlightedIndex = nil
        collapseMenu?()
        guard let top = navigation.topViewController else { return }
        moreMenuState.items[index].action?(top)
      }
      bar.onSelect = { [weak self] tab in
        self?.select(tab)
      }
      bar.onExpandedChange = { [weak self, weak createButton] expanded in
        guard let self else { return }
        if expanded { createButton?.setExpanded(false) }
        menuDismissView.isHidden = !expanded && createButton?.isExpanded != true
        moreMenuState?.isPresented = expanded
        if !expanded { moreMenuState?.highlightedIndex = nil }
      }
      createButton.onExpandedChange = { [weak self, weak bar] expanded in
        guard let self else { return }
        if expanded { bar?.setExpanded(false) }
        menuDismissView.isHidden = !expanded && bar?.isExpanded != true
      }
      bar.searchButton.addAction(
        UIAction { [weak self] _ in self?.openSearch() }, for: .primaryActionTriggered)
      bar.dismissButton.addAction(
        UIAction { [weak self] _ in self?.closeSearch() }, for: .primaryActionTriggered)
      collapseMenu = { [weak bar, weak createButton] in
        bar?.setExpanded(false)
        createButton?.setExpanded(false)
      }
      let chrome = Container.shared.bottomChrome()
      applyCreateButton = { [weak self, weak createButton] duration in
        guard let self else { return }
        let shown = createMenuAvailable && !barSearching && !barConcealed
        createButton?.setShown(shown, duration: duration)
        chrome.set(inset: barConcealed ? 0 : MainTabBar.contentBottomInset)
      }
      setBarSearching = { [weak self, weak bar] searching in
        guard let self else { return }
        barSearching = searching
        bar?.setSearching(searching)
        applyCreateButton?(nil)
      }
      setBarConcealed = { [weak self, weak bar] concealed, duration in
        guard let self else { return }
        barConcealed = concealed
        bar?.setConcealed(concealed, duration: duration)
        applyCreateButton?(duration)
      }
      syncCreateButton = { [weak self, weak createButton] top in
        guard let self else { return }
        let items = top.flatMap { createItems($0) }
        createButton?.setItems(items ?? [])
        createMenuAvailable = items != nil
        applyCreateButton?(nil)
      }
      syncCreateButton?(navigation.topViewController)
      menuDismissView.isHidden = true
      menuDismissView.addGestureRecognizer(
        UITapGestureRecognizer(target: self, action: #selector(dismissMenu)))
      bar.translatesAutoresizingMaskIntoConstraints = false
      createButton.translatesAutoresizingMaskIntoConstraints = false
      menuDismissView.translatesAutoresizingMaskIntoConstraints = false
      view.addSubview(menuDismissView)
      view.addSubview(bar)
      view.addSubview(createButton)
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
        createButton.trailingAnchor.constraint(equalTo: bar.trailingAnchor),
        createButton.bottomAnchor.constraint(
          equalTo: bar.bottomAnchor, constant: -(MainTabBar.barHeight + CreateButtonView.spacing)),
      ])
    }

    @objc private func dismissMenu() {
      collapseMenu?()
    }
  }

#endif
