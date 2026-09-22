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
    private let dialog = Container.shared.dialog()
    private let authService = Container.shared.authService()
    private let sites = Container.shared.sites()
    private let creator = Container.shared.entityCreator()

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
      presenter.navigationController?.pushViewController(viewController(for: route), animated: true)
    }

    func present(
      _ route: Route, from presenter: UIViewController,
      detents: [UISheetPresentationController.Detent] = [.large()]
    ) {
      present(viewController(for: route), from: presenter, detents: detents)
    }

    func present(
      _ controller: UIViewController, from presenter: UIViewController,
      detents: [UISheetPresentationController.Detent] = [.large()]
    ) {
      let navigation = ShellNavigationController(root: controller)
      navigation.modalPresentationStyle = .pageSheet
      navigation.sheetPresentationController?.detents = detents
      let resizable = detents.count > 1
      navigation.sheetPresentationController?.prefersGrabberVisible = resizable
      if !resizable {
        controller.navigationItem.leftBarButtonItem = UIBarButtonItem(
          title: "닫기",
          primaryAction: UIAction { [weak navigation] _ in navigation?.dismiss(animated: true) })
      }
      presenter.present(navigation, animated: true)
    }

    func pushSearchHit(_ hit: SearchHit, from presenter: UIViewController) -> UIViewController? {
      guard let navigation = presenter.navigationController else { return nil }
      let controller: UIViewController =
        switch hit {
        case .document: viewController(for: .document(entityId: hit.entityId))
        case .folder(let folder):
          folderController(
            entityId: folder.entityId, title: folder.title.plain,
            tree: Container.shared.homeTreeStore())
        }
      navigation.pushViewController(controller, animated: true)
      return controller
    }

    func pushCreateSite(from presenter: UIViewController) {
      guard let navigation = presenter.navigationController else { return }
      let model = Container.shared.createSiteModel()
      let form = ThemedHostingController(
        title: "새 스페이스 생성", CreateSiteScreen(model: model))
      let push = { [weak navigation] in
        guard let navigation else { return }
        navigation.pushViewController(form, animated: true)
        guard let coordinator = navigation.transitionCoordinator else {
          model.focusName()
          return
        }
        coordinator.animate(alongsideTransition: nil) { context in
          if !context.isCancelled { model.focusName() }
        }
      }
      guard let sheet = presenter.sheetPresentationController,
        sheet.selectedDetentIdentifier != .large
      else {
        push()
        return
      }
      CATransaction.begin()
      CATransaction.setCompletionBlock(push)
      sheet.animateChanges { sheet.selectedDetentIdentifier = .large }
      CATransaction.commit()
    }

    func pushPinnedEntities(home: HomeStore, from presenter: UIViewController) {
      let host = HostReference()
      let controller = ThemedHostingController(
        title: "고정",
        PinnedEntitiesScreen(
          store: home,
          onOpenDocument: { [weak self] entityId in
            guard let self, let presenter = host.controller else { return }
            push(.document(entityId: entityId), from: presenter)
          },
          onOpenFolder: { [weak self] item in
            guard let self, let presenter = host.controller else { return }
            pushFolder(item, tree: Container.shared.homeTreeStore(), from: presenter)
          }))
      controller.hidesBottomBarWhenPushed = true
      host.controller = controller
      presenter.navigationController?.pushViewController(controller, animated: true)
    }

    func pushRecentDocuments(from presenter: UIViewController) {
      let host = HostReference()
      let controller = ThemedHostingController(
        title: "최근",
        RecentDocumentsScreen(store: Container.shared.recentDocumentsStore()) {
          [weak self] entityId in
          guard let self, let presenter = host.controller else { return }
          push(.document(entityId: entityId), from: presenter)
        })
      controller.hidesBottomBarWhenPushed = true
      host.controller = controller
      presenter.navigationController?.pushViewController(controller, animated: true)
    }

    func pushStudioTree(home: HomeStore, tree: HomeTreeStore, from presenter: UIViewController) {
      let host = HostReference()
      let controller = ThemedHostingController(
        title: sites.current?.name ?? "스페이스",
        StudioTreeScreen(
          store: home, tree: tree,
          onOpenDocument: { [weak self] entityId in
            guard let self, let presenter = host.controller else { return }
            push(.document(entityId: entityId), from: presenter)
          },
          onOpenFolder: { [weak self] item in
            guard let self, let presenter = host.controller else { return }
            pushFolder(item, tree: tree, from: presenter)
          }))
      host.controller = controller
      controller.creationContext = EntityCreationContext(
        siteId: { home.siteId }, parentEntityId: nil, didCreate: { home.refetch() })
      presenter.navigationController?.pushViewController(controller, animated: true)
    }

    func pushFolder(_ item: EntityRowItem, tree: HomeTreeStore, from presenter: UIViewController) {
      presenter.navigationController?.pushViewController(
        folderController(entityId: item.entityId, title: item.title, tree: tree), animated: true)
    }

    func folderController(entityId: String, title: String, tree: HomeTreeStore)
      -> UIViewController
    {
      let host = HostReference()
      let controller = ThemedHostingController(
        title: title,
        FolderScreen(
          folderId: entityId, tree: tree,
          onOpenDocument: { [weak self] childId in
            guard let self, let presenter = host.controller else { return }
            push(.document(entityId: childId), from: presenter)
          },
          onOpenFolder: { [weak self] child in
            guard let self, let presenter = host.controller else { return }
            pushFolder(child, tree: tree, from: presenter)
          }))
      controller.hidesBottomBarWhenPushed = true
      host.controller = controller
      controller.creationContext = EntityCreationContext(
        siteId: { Container.shared.activeSite().siteId }, parentEntityId: entityId,
        didCreate: { tree.refetchChildren(of: entityId) })
      let add = UIBarButtonItem(image: menuIcon(LucideIcon.plus), menu: createMenu(for: controller))
      add.accessibilityLabel = "새로 만들기"
      controller.navigationItem.rightBarButtonItem = add
      return controller
    }

    func createItems(for controller: UIViewController) -> [CreateMenuItem]? {
      guard let context = controller.creationContext else { return nil }
      return [
        CreateMenuItem(icon: LucideIcon.filePlus, title: "새 문서") { [weak self, weak controller] in
          guard let self, let controller, let siteId = context.siteId() else { return }
          Task { @MainActor [weak self, weak controller] in
            guard let self, !creator.isCreating else { return }
            guard
              let entityId = await creator.createDocument(
                siteId: siteId, parentEntityId: context.parentEntityId)
            else {
              toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
              return
            }
            context.didCreate()
            guard let controller else { return }
            push(.document(entityId: entityId), from: controller)
          }
        },
        CreateMenuItem(icon: LucideIcon.folderPlus, title: "새 폴더") { [weak self] in
          guard let self, let siteId = context.siteId() else { return }
          Task { @MainActor [weak self] in
            guard let self, !creator.isCreating else { return }
            guard
              await creator.createFolder(siteId: siteId, parentEntityId: context.parentEntityId)
                != nil
            else {
              toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
              return
            }
            context.didCreate()
          }
        },
        CreateMenuItem(icon: LucideIcon.minus, title: "새 구분선") { [weak self] in
          guard let self, let siteId = context.siteId() else { return }
          Task { @MainActor [weak self] in
            guard let self, !creator.isCreating else { return }
            guard
              await creator.createDivider(siteId: siteId, parentEntityId: context.parentEntityId)
                != nil
            else {
              toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
              return
            }
            context.didCreate()
          }
        },
      ]
    }

    func createMenu(for controller: UIViewController) -> UIMenu? {
      guard let items = createItems(for: controller) else { return nil }
      return UIMenu(
        children: items.map { item in
          UIAction(title: item.title, image: menuIcon(item.icon)) { _ in item.action() }
        })
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
      let logo = SiteLogoBarItem(menu: menu, displayScale: controller.traitCollection.displayScale)
      controller.navigationItem.leftBarButtonItem = logo.item
      objc_setAssociatedObject(controller, &siteLogoKey, logo, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
      let model = sites
      keepObserving(while: logo) { [weak logo, weak model] in
        guard let logo, let model else { return }
        let source = model.current?.logo
        let url = Img.url(of: source)
        logo.currentURL = url
        let scale = logo.displayScale
        Task { @MainActor [weak logo] in
          let image = await Img.load(source, side: SiteLogoBarItem.logoSide, scale: scale)
          guard let logo, logo.currentURL == url else { return }
          logo.setImage(image)
        }
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

    func moreMenuItems() -> [MoreMenuItem] {
      [
        MoreMenuItem(icon: LucideIcon.palette, title: "디자인 쇼케이스") { [weak self] presenter in
          guard let self else { return }
          pushScreen(title: "Design", TDesignShowcase(theme: theme), from: presenter)
        },
        MoreMenuItem(icon: LucideIcon.logOut, title: "로그아웃") { [weak self] _ in
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

    private func pushScreen(title: String, _ screen: some View, from presenter: UIViewController) {
      presenter.navigationController?.pushViewController(
        ThemedHostingController(title: title, screen), animated: true)
    }

    private func viewController(for route: Route) -> UIViewController {
      let host = HostReference()
      let controller: UIViewController =
        switch route {
        case .siteSwitcher: siteSwitcherController(host: host)
        case .home: homeController(host: host)
        case .userGoal: userGoalController(host: host)
        default: placeholderController(for: route, host: host)
        }
      host.controller = controller
      return controller
    }

    private func homeController(host: HostReference) -> UIViewController {
      let home = Container.shared.homeStore()
      let tree = Container.shared.homeTreeStore()
      let title = HomeTitleState()
      let content = ThemedHostingController(
        title: "",
        HomeScreen(title: title, store: home) { [weak self] in
          HomeBody(
            store: home, tree: tree,
            onOpenGoal: { [weak self] in
              guard let self, let presenter = host.controller else { return }
              push(.userGoal, from: presenter)
            },
            onOpenPinnedAll: { [weak self] in
              guard let self, let presenter = host.controller else { return }
              pushPinnedEntities(home: home, from: presenter)
            },
            onOpenRecentAll: { [weak self] in
              guard let self, let presenter = host.controller else { return }
              pushRecentDocuments(from: presenter)
            },
            onOpenAll: { [weak self] in
              guard let self, let presenter = host.controller else { return }
              pushStudioTree(home: home, tree: tree, from: presenter)
            },
            onOpenDocument: { [weak self] entityId in
              guard let self, let presenter = host.controller else { return }
              push(.document(entityId: entityId), from: presenter)
            },
            onOpenFolder: { [weak self] item in
              guard let self, let presenter = host.controller else { return }
              pushFolder(item, tree: tree, from: presenter)
            })
        })
      let controller = HomeHostController(title: title, content: content)
      controller.creationContext = EntityCreationContext(
        siteId: { home.siteId }, parentEntityId: nil, didCreate: { home.refetch() })
      return controller
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

    private func userGoalController(host: HostReference) -> UIViewController {
      let model = Container.shared.userGoalModel()
      let controller = ThemedHostingController(
        title: "일일 목표",
        UserGoalScreen(
          model: model,
          onOpenDocument: { [weak self] entityId in
            guard let self, let presenter = host.controller else { return }
            push(.document(entityId: entityId), from: presenter)
          }))
      let edit = UIBarButtonItem(
        image: menuIcon(LucideIcon.slidersHorizontal),
        primaryAction: UIAction { [weak self] _ in
          guard let self, let presenter = host.controller else { return }
          presentUserGoalForm(goal: model, from: presenter)
        })
      edit.accessibilityLabel = "수정"
      controller.navigationItem.rightBarButtonItem = edit
      controller.hidesBottomBarWhenPushed = true
      return controller
    }

    private func presentUserGoalForm(goal: UserGoalModel, from presenter: UIViewController) {
      let model = Container.shared.userGoalFormModel(goal)
      let host = HostReference()
      let form = ThemedHostingController(
        title: model.hasGoal ? "일일 목표 수정" : "일일 목표 정하기",
        UserGoalFormScreen(model: model) { host.controller?.dismiss(animated: true) })
      host.controller = form
      let safeBottom = presenter.view.window?.safeAreaInsets.bottom ?? 0
      let height = UserGoalFormScreen.contentHeight + safeBottom
      present(
        form, from: presenter,
        detents: [.custom(identifier: .init("userGoalForm")) { _ in height }])
      form.navigationController?.sheetPresentationController?.prefersGrabberVisible = true
      form.navigationItem.leftBarButtonItem = UIBarButtonItem(
        systemItem: .close,
        primaryAction: UIAction { [weak host] _ in host?.controller?.dismiss(animated: true) })
      guard model.hasGoal else { return }
      let remove = UIBarButtonItem(
        title: "제거",
        primaryAction: UIAction { [weak self, weak host] _ in
          guard let self else { return }
          Task { await self.removeUserGoal(model, host: host) }
        })
      remove.tintColor = .theme(\.dangerDefault)
      form.navigationItem.rightBarButtonItem = remove
    }

    private func removeUserGoal(_ model: UserGoalFormModel, host: HostReference?) async {
      let confirmed = await dialog.confirm(
        TDialogItem(
          title: "일일 목표를 제거하시겠어요?", message: "설정한 하루 목표 글자 수가 사라져요.",
          confirmText: "제거", cancelText: "취소", confirmIsDestructive: true))
      guard confirmed else { return }
      if await model.remove() {
        toast.success("일일 목표를 제거했어요.")
        host?.controller?.dismiss(animated: true)
      } else {
        toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      }
    }

    private func placeholderController(for route: Route, host: HostReference) -> UIViewController {
      ThemedHostingController(
        title: String(describing: route).components(separatedBy: "(")[0],
        PlaceholderScreen(route: route))
    }
  }

  @MainActor
  private final class HostReference {
    weak var controller: UIViewController?
  }

  private nonisolated(unsafe) var siteLogoKey: UInt8 = 0

#endif
