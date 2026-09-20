#if canImport(UIKit)

  import Design
  import FactoryKit
  import UIKit

  final class AuthFlowController: UIViewController, UINavigationControllerDelegate {
    private var navigation: UINavigationController?
    private var login: UIViewController?
    private var emailModel: EmailLoginModel?

    init() {
      super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func viewDidLoad() {
      super.viewDidLoad()
      view.backgroundColor = .theme(\.surfaceCanvas)

      let login = ThemedHostingController(
        title: "",
        LoginScreen(
          onEmail: { [weak self] in self?.showEmailLogin() },
          presenter: { [weak self] in self?.topPresenter }))
      let navigation = ShellNavigationController(root: login)
      navigation.delegate = self
      applyTransparentBar(to: login, in: navigation)
      self.navigation = navigation
      self.login = login
      embed(navigation)
    }

    override func viewDidLayoutSubviews() {
      super.viewDidLayoutSubviews()
      guard let navigation, let login else { return }
      let barHeight = navigation.navigationBar.frame.height
      if login.additionalSafeAreaInsets.top != -barHeight {
        login.additionalSafeAreaInsets.top = -barHeight
      }
    }

    private var topPresenter: UIViewController {
      var controller: UIViewController = self
      while let presented = controller.presentedViewController {
        controller = presented
      }
      return controller
    }

    private func showEmailLogin() {
      guard let navigation, navigation.viewControllers.count == 1,
        navigation.transitionCoordinator == nil
      else { return }
      let model = Container.shared.emailLoginModel {}
      emailModel = model
      let screen = ThemedHostingController(title: "", EmailLoginScreen(model: model))
      applyTransparentBar(to: screen, in: navigation)
      navigation.pushViewController(screen, animated: true)
      guard let coordinator = navigation.transitionCoordinator else {
        model.focusEmail()
        return
      }
      coordinator.animate(alongsideTransition: nil) { context in
        if !context.isCancelled { model.focusEmail() }
      }
    }

    func navigationController(
      _ navigationController: UINavigationController, willShow viewController: UIViewController,
      animated: Bool
    ) {
      guard viewController === login, let model = emailModel else { return }
      let commit = { [weak self] in
        model.discardPassword()
        self?.emailModel = nil
      }
      guard let coordinator = navigationController.transitionCoordinator, coordinator.isInteractive
      else {
        commit()
        return
      }
      coordinator.notifyWhenInteractionChanges { context in
        if !context.isCancelled { commit() }
      }
    }

    private func applyTransparentBar(
      to controller: UIViewController, in navigation: UINavigationController
    ) {
      let appearance = navigation.navigationBar.standardAppearance.copy()
      appearance.configureWithTransparentBackground()
      controller.navigationItem.standardAppearance = appearance
      controller.navigationItem.scrollEdgeAppearance = appearance
    }
  }

#endif
