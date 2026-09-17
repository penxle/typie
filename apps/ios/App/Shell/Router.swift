import Core
import Design
import SwiftUI
import UIKit

@MainActor
final class Router {
  private let theme: ThemeSettings
  private let services: CoreServices

  init(theme: ThemeSettings, services: CoreServices) {
    self.theme = theme
    self.services = services
  }

  func root(for tab: MainTab) -> UIViewController {
    let controller = viewController(for: tab.route)
    controller.title = tab.label
    if tab == .home {
      controller.navigationItem.leftBarButtonItem = ProfileAvatar.barButtonItem()
    }
    return controller
  }

  func push(_ route: Route, from presenter: UIViewController) {
    presenter.navigationController?.pushViewController(viewController(for: route), animated: true)
  }

  func present(_ route: Route, from presenter: UIViewController) {
    let destination = viewController(for: route)
    let navigation = ShellNavigationController.make(root: destination)
    navigation.modalPresentationStyle = .pageSheet
    destination.navigationItem.leftBarButtonItem = UIBarButtonItem(
      title: "닫기",
      primaryAction: UIAction { [weak navigation] _ in navigation?.dismiss(animated: true) })
    presenter.present(navigation, animated: true)
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
          CreateMenuItem(title: "여기에 폴더 만들기", image: icon(LucideIcon.folderPlus)) {
            [weak self] presenter in
            self?.push(.folder(entityId: "new"), from: presenter)
          },
          CreateMenuItem(title: "여기에 문서 만들기", image: icon(LucideIcon.squarePen)) {
            [weak self] presenter in
            self?.push(.document(entityId: "new"), from: presenter)
          },
        ]))
    case .notes:
      CreateAction(label: "새 노트", image: icon(TypieIcon.stickyNotePlus), kind: .perform { _ in })
    }
  }

  private func icon(_ name: TIconName) -> UIImage? {
    UIImage(named: name.assetName, in: DesignBundle.bundle, with: nil)
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
    let actions: [PlaceholderAction] =
      switch route {
      case .home: homeActions(host: host)
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
    let controller = ThemedHostingController.make(
      title: String(describing: route).components(separatedBy: "(")[0],
      PlaceholderScreen(route: route, actions: actions, fillerCount: route == .home ? 40 : 0))
    host.controller = controller
    return controller
  }
}

@MainActor
private final class HostReference {
  weak var controller: UIViewController?
}
