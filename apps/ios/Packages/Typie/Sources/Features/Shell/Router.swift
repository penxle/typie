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

    func tabRoot() -> TabRootController {
      let root = TabRootController(initial: MainTab.initial) { tab in self.root(for: tab) }
      installSiteLogo(on: root)
      let profile = UIBarButtonItem(
        image: menuIcon(LucideIcon.circleUserRound),
        primaryAction: UIAction { [weak self, weak root] _ in
          guard let self, let root else { return }
          push(.profile, from: root)
        })
      profile.accessibilityLabel = "프로필 메뉴"
      root.sharedRightItems = [profile]
      return root
    }

    private func root(for tab: MainTab) -> UIViewController {
      let controller = viewController(for: tab.route)
      controller.title = tab.label
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
            entityId: folder.entityId,
            initial: EntityFolderItem(
              entityId: folder.entityId, icon: folder.icon, path: folder.path,
              title: folder.title.plain, folderCount: folder.folderCount,
              documentCount: folder.documentCount),
            title: folder.title.plain)
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
            guard let self, let presenter = host.controller, case .folder(let folder) = item
            else { return }
            pushFolder(folder, from: presenter)
          }))
      controller.hidesBottomBarWhenPushed = true
      host.controller = controller
      presenter.navigationController?.pushViewController(controller, animated: true)
    }

    private func presentHomeCustomize(from presenter: UIViewController) {
      let screen = ThemedHostingController(
        title: "홈 사용자화", HomeCustomizeScreen(layout: Container.shared.homeLayoutStore()))
      present(screen, from: presenter)
      let navigation = screen.navigationController
      navigation?.sheetPresentationController?.detents = [
        .custom(identifier: .init("homeCustomize")) { [weak navigation] _ in
          HomeCustomizeScreen.contentHeight + (navigation?.navigationBar.bounds.height ?? 54)
        }
      ]
      screen.navigationItem.leftBarButtonItem = closeItem { [weak navigation] in
        navigation?.dismiss(animated: true)
      }
      navigation?.sheetPresentationController?.prefersGrabberVisible = true
    }

    func pushRecentDocuments(from presenter: UIViewController) {
      let host = HostReference()
      let store = Container.shared.recentDocumentsStore()
      let controller = ThemedHostingController(
        title: "최근",
        RecentDocumentsScreen(store: store) { [weak self] entityId in
          guard let self, let presenter = host.controller else { return }
          push(.document(entityId: entityId), from: presenter)
        })
      let sortMenu = UIMenu(
        title: "정렬 기준",
        children: [
          UIDeferredMenuElement.uncached { [weak store] completion in
            let current = store?.sort
            completion(
              RecentSort.allCases.map { sort in
                UIAction(title: sort.title, state: current == sort ? .on : .off) { [weak store] _ in
                  store?.sortOverride = sort
                }
              })
          }
        ])
      let more = UIBarButtonItem(image: menuIcon(LucideIcon.ellipsis), menu: sortMenu)
      more.accessibilityLabel = "더 보기"
      controller.navigationItem.rightBarButtonItem = more
      controller.hidesBottomBarWhenPushed = true
      host.controller = controller
      presenter.navigationController?.pushViewController(controller, animated: true)
    }

    func pushSiteEntities(home: HomeStore, from presenter: UIViewController) {
      let host = HostReference()
      let store = Container.shared.siteEntitiesStore()
      let title = HeroTitleState()
      let name = sites.current?.name
      let content = ThemedHostingController(
        title: "",
        SiteEntitiesScreen(
          store: store, title: title, initialName: name,
          onOpenDocument: { [weak self] entityId in
            guard let self, let presenter = host.controller else { return }
            push(.document(entityId: entityId), from: presenter)
          },
          onOpenFolder: { [weak self] folder in
            guard let self, let presenter = host.controller else { return }
            pushFolder(folder, from: presenter)
          }))
      let controller = HeroHostController(title: title, content: content)
      controller.title = name ?? "스페이스"
      controller.hidesBottomBarWhenPushed = true
      controller.keepsCreateButtonWhenBarHidden = true
      controller.navigationItem.rightBarButtonItems = [moreItem(), homeItem(for: controller)]
      host.controller = controller
      controller.creationContext = EntityCreationContext(
        siteId: { home.siteId }, parentEntityId: nil,
        didCreate: { [weak store] in
          store?.refetch()
          home.refetch()
        })
      presenter.navigationController?.pushViewController(controller, animated: true)
    }

    func pushFolder(_ folder: EntityFolderItem, from presenter: UIViewController) {
      presenter.navigationController?.pushViewController(
        folderController(entityId: folder.entityId, initial: folder, title: folder.title),
        animated: true)
    }

    func folderController(entityId: String, initial: EntityFolderItem?, title: String)
      -> UIViewController
    {
      let host = HostReference()
      let store = FolderContentsStore(entityId: entityId, initial: initial, title: title)
      let titleState = HeroTitleState()
      let content = ThemedHostingController(
        title: "",
        FolderScreen(
          store: store, title: titleState,
          onOpenDocument: { [weak self] childId in
            guard let self, let presenter = host.controller else { return }
            push(.document(entityId: childId), from: presenter)
          },
          onOpenFolder: { [weak self] child in
            guard let self, let presenter = host.controller else { return }
            pushFolder(child, from: presenter)
          }))
      let controller = HeroHostController(title: titleState, content: content)
      controller.title = title
      controller.hidesBottomBarWhenPushed = true
      controller.keepsCreateButtonWhenBarHidden = true
      controller.navigationItem.rightBarButtonItems = [moreItem(), homeItem(for: controller)]
      host.controller = controller
      controller.creationContext = EntityCreationContext(
        siteId: { Container.shared.activeSite().siteId }, parentEntityId: entityId,
        didCreate: { [weak store] in
          store?.refetch()
          Container.shared.homeTreeStore().refetchChildren(of: entityId)
        })
      return controller
    }

    private func homeItem(for controller: UIViewController) -> UIBarButtonItem {
      let item = UIBarButtonItem(
        image: menuIcon(LucideIcon.house),
        primaryAction: UIAction { [weak controller] _ in
          controller?.navigationController?.popToRootViewController(animated: true)
        })
      item.accessibilityLabel = "홈"
      return item
    }

    private func moreItem() -> UIBarButtonItem {
      let item = UIBarButtonItem(
        image: menuIcon(LucideIcon.ellipsis), primaryAction: UIAction { _ in })
      item.accessibilityLabel = "더 보기"
      return item
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

    private func closeItem(_ dismiss: @escaping @MainActor () -> Void) -> UIBarButtonItem {
      let item = UIBarButtonItem(
        image: menuIcon(LucideIcon.x), primaryAction: UIAction { _ in dismiss() })
      item.accessibilityLabel = "닫기"
      return item
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

    private func viewController(for route: Route) -> UIViewController {
      let host = HostReference()
      let controller: UIViewController =
        switch route {
        case .siteSwitcher: siteSwitcherController(host: host)
        case .home: homeController(host: host)
        case .userGoal: userGoalController(host: host)
        case .profile: profileController()
        default: placeholderController(for: route, host: host)
        }
      host.controller = controller
      return controller
    }

    private func homeController(host: HostReference) -> UIViewController {
      let home = Container.shared.homeStore()
      let tree = Container.shared.homeTreeStore()
      let title = HeroTitleState()
      let content = ThemedHostingController(
        title: "",
        HomeScreen(title: title, store: home) { [weak self] in
          HomeBody(
            store: home, tree: tree, layout: Container.shared.homeLayoutStore(),
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
              pushSiteEntities(home: home, from: presenter)
            },
            onOpenDocument: { [weak self] entityId in
              guard let self, let presenter = host.controller else { return }
              push(.document(entityId: entityId), from: presenter)
            },
            onOpenFolder: { [weak self] item in
              guard let self, let presenter = host.controller, case .folder(let folder) = item
              else { return }
              pushFolder(folder, from: presenter)
            })
        })
      let controller = HeroHostController(title: title, content: content)
      let more = UIBarButtonItem(
        image: menuIcon(LucideIcon.ellipsis),
        menu: UIMenu(children: [
          UIAction(title: "홈 사용자화", image: menuIcon(LucideIcon.slidersHorizontal)) {
            [weak self, weak controller] _ in
            guard let self, let controller else { return }
            presentHomeCustomize(from: controller)
          }
        ]))
      more.accessibilityLabel = "더 보기"
      controller.navigationItem.rightBarButtonItem = more
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

    private func profileController() -> UIViewController {
      let controller = ThemedHostingController(
        title: "프로필",
        ProfileScreen(model: Container.shared.profileModel()) { [weak self] in
          guard let self else { return }
          Task { await self.logout() }
        })
      controller.hidesBottomBarWhenPushed = true
      return controller
    }

    private func logout() async {
      let confirmed = await dialog.confirm(
        TDialogItem(
          title: "로그아웃", message: "정말 로그아웃하시겠어요?", confirmText: "로그아웃",
          cancelText: "취소", confirmIsDestructive: true))
      guard confirmed else { return }
      do {
        try await authService.logout()
      } catch {
        toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      }
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
      form.navigationItem.leftBarButtonItem = closeItem { [weak host] in
        host?.controller?.dismiss(animated: true)
      }
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
