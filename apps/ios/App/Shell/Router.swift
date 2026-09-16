import Core
import Design
import UIKit

@MainActor
final class Router {
  private let theme: ThemeSettings

  init(theme: ThemeSettings) {
    self.theme = theme
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

  func showDesignShowcase(from presenter: UIViewController) {
    let controller = ThemedHostingController.make(title: "Design", DesignShowcase(theme: theme))
    presenter.navigationController?.pushViewController(controller, animated: true)
  }

  private func viewController(for route: Route) -> UIViewController {
    let host = HostReference()
    let actions: [PlaceholderAction] =
      switch route {
      case .home:
        [
          PlaceholderAction(title: "push settings") { [weak self] in
            guard let presenter = host.controller else { return }
            self?.push(.settings, from: presenter)
          },
          PlaceholderAction(title: "push document") { [weak self] in
            guard let presenter = host.controller else { return }
            self?.push(.document(entityId: "sample"), from: presenter)
          },
          PlaceholderAction(title: "push design showcase") { [weak self] in
            guard let presenter = host.controller else { return }
            self?.showDesignShowcase(from: presenter)
          },
        ]
      case .document:
        [
          PlaceholderAction(title: "present body settings") { [weak self] in
            guard let presenter = host.controller else { return }
            self?.present(.documentBodySettings(entityId: "sample"), from: presenter)
          }
        ]
      case .studio:
        [
          PlaceholderAction(title: "push folder") { [weak self] in
            guard let presenter = host.controller else { return }
            self?.push(.folder(entityId: "sample"), from: presenter)
          },
          PlaceholderAction(title: "present folder details") { [weak self] in
            guard let presenter = host.controller else { return }
            self?.present(.folderDetails(entityId: "sample"), from: presenter)
          },
        ]
      case .folder:
        [
          PlaceholderAction(title: "push folder") { [weak self] in
            guard let presenter = host.controller else { return }
            self?.push(.folder(entityId: "nested"), from: presenter)
          }
        ]
      default:
        []
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
