#if canImport(UIKit)

  import Core
  import Design
  import UIKit

  final class MainTabBarController: UITabBarController, UITabBarControllerDelegate {
    private static let overlayButtonSide: CGFloat = 52
    private static let overlayButtonMargin: CGFloat = 16

    private let createAction: @MainActor (MainTab) -> CreateAction?
    private var createButton: UIButton?
    private let menuDismissView = UIView()
    private var collapseMenu: (() -> Void)?
    private var moreMenuState: MoreMenuState?
    private let selectionFeedback = UISelectionFeedbackGenerator()
    private var performCreate: (@MainActor (UIViewController) -> Void)?

    init(
      rootProvider: @escaping @MainActor (MainTab) -> UIViewController,
      createAction: @escaping @MainActor (MainTab) -> CreateAction?
    ) {
      self.createAction = createAction
      super.init(nibName: nil, bundle: nil)
      tabs = MainTab.allCases.map { tab in
        UITab(
          title: tab.label, image: tab.image, identifier: tab.rawValue
        ) { _ in
          ShellNavigationController(root: rootProvider(tab))
        }
      }
      tabBar.tintColor = .theme(\.textDefault)
      delegate = self
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func viewDidLoad() {
      super.viewDidLoad()
      let button: UIButton
      if #available(iOS 26, *) {
        button = installCustomBar()
      } else {
        button = installOverlayButton()
      }
      button.addAction(
        UIAction { [weak self] _ in
          guard let self, let presenter else { return }
          performCreate?(presenter)
        }, for: .primaryActionTriggered)
      createButton = button
      configureCreateButton(for: .initial)
    }

    @available(iOS 26, *)
    private func installCustomBar() -> UIButton {
      isTabBarHidden = true
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
      addChild(menuHost)
      let bar = MainTabBar(
        selectedTint: .theme(\.textDefault), normalTint: .theme(\.textMuted),
        accentTint: .theme(\.paletteBlue), menuView: menuHost.view,
        menuHeaderHeight: MoreMenu.headerHeight, menuRowHeight: MoreMenu.rowHeight,
        menuRowCount: menuState.items.count)
      menuHost.didMove(toParent: self)
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
        guard let self else { return }
        selectedTab = tabs[tab.index]
        configureCreateButton(for: tab)
      }
      bar.onExpandedChange = { [weak self] expanded in
        self?.menuDismissView.isHidden = !expanded
        self?.moreMenuState?.isPresented = expanded
        if !expanded { self?.moreMenuState?.highlightedIndex = nil }
      }
      collapseMenu = { [weak bar] in bar?.setExpanded(false) }
      menuDismissView.isHidden = true
      menuDismissView.addGestureRecognizer(
        UITapGestureRecognizer(target: self, action: #selector(dismissMenu)))
      bar.translatesAutoresizingMaskIntoConstraints = false
      menuDismissView.translatesAutoresizingMaskIntoConstraints = false
      view.addSubview(menuDismissView)
      view.addSubview(bar)
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
          equalTo: view.bottomAnchor, constant: -MainTabBar.bottomPadding),
      ])
      return bar.createButton
    }

    @objc private func dismissMenu() {
      collapseMenu?()
    }

    private func installOverlayButton() -> UIButton {
      let button = UIButton(type: .system)
      var configuration = UIButton.Configuration.filled()
      configuration.cornerStyle = .capsule
      configuration.baseForegroundColor = .theme(\.textDefault)
      button.configuration = configuration
      view.addSubview(button)
      return button
    }

    override func viewDidLayoutSubviews() {
      super.viewDidLayoutSubviews()
      if #available(iOS 26, *) {
        let inset = max(0, MainTabBar.contentBottomInset - view.safeAreaInsets.bottom)
        if selectedViewController?.additionalSafeAreaInsets.bottom != inset {
          selectedViewController?.additionalSafeAreaInsets.bottom = inset
        }
        return
      }
      guard let createButton else { return }
      let side = Self.overlayButtonSide
      let safeArea = view.safeAreaLayoutGuide.layoutFrame
      let barFrame = tabBar.superview.map { view.convert(tabBar.frame, from: $0) }
      let centerY = barFrame?.midY ?? (safeArea.maxY - Self.overlayButtonMargin - side / 2)
      createButton.frame = CGRect(
        x: safeArea.maxX - Self.overlayButtonMargin - side, y: centerY - side / 2, width: side,
        height: side)
    }

    func tabBarController(
      _ tabBarController: UITabBarController, didSelectTab selectedTab: UITab, previousTab: UITab?
    ) {
      guard let mainTab = MainTab(rawValue: selectedTab.identifier) else { return }
      configureCreateButton(for: mainTab)
    }

    private func configureCreateButton(for tab: MainTab) {
      guard let createButton else { return }
      guard let action = createAction(tab) else {
        createButton.isHidden = true
        return
      }
      createButton.isHidden = false
      let image = createButtonImage(action.image)
      if createButton.configuration?.image != nil, createButton.configuration?.image != image {
        UIView.transition(with: createButton, duration: 0.1, options: .transitionCrossDissolve) {
          createButton.configuration?.image = image
        }
      } else {
        createButton.configuration?.image = image
      }
      createButton.accessibilityLabel = action.label
      switch action.kind {
      case .perform(let perform):
        performCreate = perform
        createButton.menu = nil
        createButton.showsMenuAsPrimaryAction = false
      case .menu(let items):
        performCreate = nil
        createButton.menu = UIMenu(
          children: items.map { item in
            UIAction(title: item.title, image: item.image) { [weak self] _ in
              guard let presenter = self?.presenter else { return }
              item.perform(presenter)
            }
          })
        createButton.showsMenuAsPrimaryAction = true
      }
    }

    private func createButtonImage(_ image: UIImage?) -> UIImage? {
      guard #available(iOS 26, *), let image else { return image }
      let size = CGSize(width: MainTabBar.iconSide, height: MainTabBar.iconSide)
      return UIGraphicsImageRenderer(size: size).image { _ in
        image.draw(in: CGRect(origin: .zero, size: size))
      }.withRenderingMode(.alwaysTemplate)
    }

    private var presenter: UIViewController? {
      (selectedTab?.viewController as? UINavigationController)?.topViewController
    }
  }

  extension MainTab {
    var label: String {
      switch self {
      case .home: "홈"
      case .studio: "스페이스"
      case .notes: "노트"
      }
    }

    var image: UIImage {
      let name =
        switch self {
        case .home: LucideIcon.house
        case .studio: LucideIcon.folderOpen
        case .notes: LucideIcon.stickyNote
        }
      return UIImage(named: name.assetName, in: TDesignBundle.bundle, with: nil)!
    }

    var selectedImage: UIImage {
      let name =
        switch self {
        case .home: TypieIcon.houseFilled
        case .studio: TypieIcon.folderOpenFilled
        case .notes: TypieIcon.stickyNoteFilled
        }
      return UIImage(named: name.assetName, in: TDesignBundle.bundle, with: nil)!
    }
  }

#endif
