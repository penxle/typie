import Core
import Design
import Home
import SwiftUI
import UIKit

@MainActor
final class Router {
  private let theme: ThemeSettings
  private let services: CoreServices
  private let spaceSwitcher: SpaceSwitcherModel
  private let toast: TToastCenter

  init(
    theme: ThemeSettings, services: CoreServices, spaceSwitcher: SpaceSwitcherModel,
    toast: TToastCenter
  ) {
    self.theme = theme
    self.services = services
    self.spaceSwitcher = spaceSwitcher
    self.toast = toast
  }

  func root(for tab: MainTab) -> UIViewController {
    let controller = viewController(for: tab.route)
    controller.title = tab.label
    if tab == .home {
      installSpaceLogo(on: controller)
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
    let navigation = ShellNavigationController.make(root: destination)
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

  func pushSpaceCreate(from presenter: UIViewController) {
    guard let navigation = presenter.navigationController else { return }
    let model = SpaceCreateModel(
      create: { [spaceSwitcher] in await spaceSwitcher.createSpace(name: $0) })
    let form = ThemedHostingController.make(
      title: "새 스페이스 생성", SpaceCreateScreen(model: model, toast: toast))
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

  private func installSpaceLogo(on controller: UIViewController) {
    let menu = UIMenu(children: [
      UIAction(title: "스페이스 전환하기", image: menuIcon(LucideIcon.arrowRightLeft)) {
        [weak self, weak controller] _ in
        guard let self, let controller else { return }
        present(.spaceSwitcher, from: controller, detents: [.medium(), .large()])
      },
      UIMenu(
        options: .displayInline,
        children: [
          UIAction(title: "설정", image: menuIcon(LucideIcon.settings)) { _ in },
          UIAction(title: "휴지통", image: menuIcon(LucideIcon.trash2)) { _ in },
        ]),
    ])
    let logo = SpaceLogoBarItem(menu: menu)
    controller.navigationItem.leftBarButtonItem = logo.item
    objc_setAssociatedObject(controller, &spaceLogoKey, logo, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
    let model = spaceSwitcher
    keepObserving(while: logo) { [weak logo] in
      let url = model.current?.logo
      guard let logo else { return }
      logo.currentURL = url
      let scale = logo.displayScale
      Task { @MainActor [weak logo] in
        var image: UIImage?
        if let url {
          image = await TImage.load(url, side: SpaceLogoBarItem.side, scale: scale)
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
    UIImage(named: name.assetName, in: DesignBundle.bundle, with: nil)
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
      ThemedHostingController.make(title: title, screen), animated: true)
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
    var actions = [
      pushAction("push settings", .settings, host: host),
      pushAction("push document", .document(entityId: "sample"), host: host),
      PlaceholderAction(title: "push design showcase") { [weak self] in
        guard let self, let presenter = host.controller else { return }
        pushScreen(title: "Design", DesignShowcase(theme: theme), from: presenter)
      },
      PlaceholderAction(title: "logout") { [weak self] in
        guard let authService = self?.services.authService else { return }
        Task { await authService.logout() }
      },
    ]
    if let probe = services.serverProbe {
      let authState = services.authState
      actions.append(
        PlaceholderAction(title: "push server probe") { [weak self] in
          guard let presenter = host.controller else { return }
          self?.pushScreen(
            title: "server probe", ServerProbeScreen(probe: probe, authState: authState),
            from: presenter)
        })
    }
    return actions
  }

  private func viewController(for route: Route) -> UIViewController {
    let host = HostReference()
    let controller: UIViewController =
      switch route {
      case .spaceSwitcher: spaceSwitcherController(host: host)
      case .home: homeController(host: host)
      default: placeholderController(for: route, host: host)
      }
    host.controller = controller
    return controller
  }

  private func homeController(host: HostReference) -> UIViewController {
    let actions = homeActions(host: host)
    let model = SearchModel(
      client: services.client, activeSite: services.activeSite,
      preferences: services.preferences)
    let search = HomeSearchState()
    let content = ThemedHostingController.make(
      title: "",
      HomeScreen(
        model: model, search: search, spaces: spaceSwitcher,
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

  private func spaceSwitcherController(host: HostReference) -> UIViewController {
    ThemedHostingController.make(
      title: "스페이스",
      SpaceSwitcherScreen(
        model: spaceSwitcher,
        onSelected: { host.controller?.dismiss(animated: true) },
        onCreate: { [weak self] in
          guard let self, let presenter = host.controller else { return }
          pushSpaceCreate(from: presenter)
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
    return ThemedHostingController.make(
      title: String(describing: route).components(separatedBy: "(")[0],
      PlaceholderScreen(route: route, actions: actions))
  }
}

@MainActor
private final class HostReference {
  weak var controller: UIViewController?
}

private nonisolated(unsafe) var spaceLogoKey: UInt8 = 0
