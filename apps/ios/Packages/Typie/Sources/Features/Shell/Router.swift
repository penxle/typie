#if canImport(UIKit)

  import Core
  import Design
  import FactoryKit
  import SwiftUI
  import UIKit

  @MainActor
  final class Router {
    private let theme = Container.shared.theme()
    private let toast = Container.shared.toast()
    private let authService = Container.shared.authService()
    private let sites = Container.shared.sites()

    init() {}

    func root(for tab: MainTab) -> UIViewController {
      let controller = viewController(for: tab.route)
      controller.title = tab.label
      if tab == .home {
        installSiteLogo(on: controller)
      }
      return controller
    }

    func push(_ route: Route, from presenter: UIViewController) {
      (presenter as? HomeHostController)?.prepareForPush()
      presenter.navigationController?.pushViewController(viewController(for: route), animated: true)
    }

    func present(
      _ route: Route, from presenter: UIViewController,
      detents: [UISheetPresentationController.Detent] = [.large()]
    ) {
      let destination = viewController(for: route)
      destination.view.backgroundColor = .clear
      let navigation = ShellNavigationController(root: destination)
      navigation.modalPresentationStyle = .pageSheet
      navigation.sheetPresentationController?.detents = detents
      let resizable = detents.count > 1
      navigation.sheetPresentationController?.prefersGrabberVisible = resizable
      if !resizable {
        destination.navigationItem.leftBarButtonItem = UIBarButtonItem(
          title: "닫기",
          primaryAction: UIAction { [weak navigation] _ in navigation?.dismiss(animated: true) })
      }
      presenter.present(navigation, animated: true)
    }

    func pushCreateSite(from presenter: UIViewController) {
      guard let navigation = presenter.navigationController else { return }
      let model = Container.shared.createSiteModel()
      let form = ThemedHostingController(
        title: "새 스페이스 생성", CreateSiteScreen(model: model))
      form.view.backgroundColor = .clear
      navigation.pushViewController(form, animated: true)
      if let sheet = presenter.sheetPresentationController {
        sheet.animateChanges { sheet.selectedDetentIdentifier = .large }
      }
      guard let coordinator = navigation.transitionCoordinator else {
        model.focusName()
        return
      }
      coordinator.animate(alongsideTransition: nil) { context in
        if !context.isCancelled { model.focusName() }
      }
    }

    private func installSiteLogo(on controller: UIViewController) {
      let menu = UIMenu(children: [
        UIAction(title: "스페이스 전환하기", image: menuIcon(LucideIcon.arrowRightLeft)) {
          [weak self, weak controller] _ in
          guard let self, let controller else { return }
          present(.siteSwitcher, from: controller, detents: [.medium(), .large()])
        },
        UIMenu(
          options: .displayInline,
          children: [
            UIAction(title: "설정", image: menuIcon(LucideIcon.settings)) { _ in },
            UIAction(title: "휴지통", image: menuIcon(LucideIcon.trash2)) { _ in },
          ]),
      ])
      let logo = SiteLogoBarItem(menu: menu)
      controller.navigationItem.leftBarButtonItem = logo.item
      objc_setAssociatedObject(controller, &siteLogoKey, logo, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
      let model = sites
      keepObserving(while: logo) { [weak logo, weak model] in
        guard let logo, let model else { return }
        let url = model.current?.logo
        logo.currentURL = url
        let scale = logo.displayScale
        Task { @MainActor [weak logo] in
          var image: UIImage?
          if let url {
            image = await TImage.load(url, side: SiteLogoBarItem.side, scale: scale)
          }
          guard let logo, logo.currentURL == url else { return }
          logo.setImage(image)
        }
      }
    }

    func createAction(for tab: MainTab) -> CreateAction? {
      switch tab {
      case .home:
        CreateAction(
          label: "새 문서", image: icon(LucideIcon.pencil),
          kind: .perform { [weak self] presenter in
            self?.push(.document(entityId: "new"), from: presenter)
          })
      case .studio:
        CreateAction(
          label: "새로 만들기", image: icon(LucideIcon.squarePlus),
          kind: .menu([
            CreateMenuItem(title: "여기에 폴더 만들기", image: menuIcon(LucideIcon.folderPlus)) {
              [weak self] presenter in
              self?.push(.folder(entityId: "new"), from: presenter)
            },
            CreateMenuItem(title: "여기에 문서 만들기", image: menuIcon(LucideIcon.squarePen)) {
              [weak self] presenter in
              self?.push(.document(entityId: "new"), from: presenter)
            },
          ]))
      case .notes:
        CreateAction(label: "새 노트", image: icon(TypieIcon.stickyNotePlus), kind: .perform { _ in })
      }
    }

    private static let menuIconSide: CGFloat = 20

    private func icon(_ name: TIconName) -> UIImage? {
      UIImage(named: name.assetName, in: TDesignBundle.bundle, with: nil)
    }

    private func menuIcon(_ name: TIconName) -> UIImage? {
      guard let image = icon(name) else { return nil }
      let size = CGSize(width: Self.menuIconSide, height: Self.menuIconSide)
      return UIGraphicsImageRenderer(size: size).image { _ in
        image.draw(in: CGRect(origin: .zero, size: size))
      }.withRenderingMode(.alwaysTemplate)
    }

    private func pushScreen(title: String, _ screen: some View, from presenter: UIViewController) {
      presenter.navigationController?.pushViewController(
        ThemedHostingController(title: title, screen), animated: true)
    }

    private func pushAction(_ title: String, _ route: Route, host: HostReference)
      -> PlaceholderAction
    {
      PlaceholderAction(title: title) { [weak self] in
        guard let presenter = host.controller else { return }
        self?.push(route, from: presenter)
      }
    }

    private func presentAction(_ title: String, _ route: Route, host: HostReference)
      -> PlaceholderAction
    {
      PlaceholderAction(title: title) { [weak self] in
        guard let presenter = host.controller else { return }
        self?.present(route, from: presenter)
      }
    }

    private func homeActions(host: HostReference) -> [PlaceholderAction] {
      [
        pushAction("push settings", .settings, host: host),
        pushAction("push document", .document(entityId: "sample"), host: host),
        PlaceholderAction(title: "push design showcase") { [weak self] in
          guard let self, let presenter = host.controller else { return }
          pushScreen(title: "Design", TDesignShowcase(theme: theme), from: presenter)
        },
        PlaceholderAction(title: "logout") { [weak self] in
          guard let self else { return }
          Task { [authService, toast] in
            do {
              try await authService.logout()
            } catch {
              toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
            }
          }
        },
      ]
    }

    private func viewController(for route: Route) -> UIViewController {
      let host = HostReference()
      let controller: UIViewController =
        switch route {
        case .siteSwitcher: siteSwitcherController(host: host)
        case .home: homeController(host: host)
        default: placeholderController(for: route, host: host)
        }
      host.controller = controller
      return controller
    }

    private func homeController(host: HostReference) -> UIViewController {
      let actions = homeActions(host: host)
      let search = HomeSearchState()
      let content = ThemedHostingController(
        title: "",
        HomeScreen(
          search: search,
          onOpen: { [weak self] hit in
            guard let self, let presenter = host.controller else { return }
            switch hit {
            case .document: push(.document(entityId: hit.entityId), from: presenter)
            case .folder: push(.folder(entityId: hit.entityId), from: presenter)
            }
          },
          extra: { PlaceholderDevActions(actions: actions) }))
      return HomeHostController(search: search, content: content)
    }

    private func siteSwitcherController(host: HostReference) -> UIViewController {
      ThemedHostingController(
        title: "스페이스",
        SiteSwitcherScreen(
          onSelected: { host.controller?.dismiss(animated: true) },
          onCreate: { [weak self] in
            guard let self, let presenter = host.controller else { return }
            pushCreateSite(from: presenter)
          }))
    }

    private func placeholderController(for route: Route, host: HostReference) -> UIViewController {
      let actions: [PlaceholderAction] =
        switch route {
        case .document:
          [
            presentAction(
              "present body settings", .documentBodySettings(entityId: "sample"), host: host)
          ]
        case .studio:
          [
            pushAction("push folder", .folder(entityId: "sample"), host: host),
            presentAction("present folder details", .folderDetails(entityId: "sample"), host: host),
          ]
        case .folder: [pushAction("push folder", .folder(entityId: "nested"), host: host)]
        default: []
        }
      return ThemedHostingController(
        title: String(describing: route).components(separatedBy: "(")[0],
        PlaceholderScreen(route: route, actions: actions))
    }
  }

  @MainActor
  private final class HostReference {
    weak var controller: UIViewController?
  }

  private nonisolated(unsafe) var siteLogoKey: UInt8 = 0

#endif
