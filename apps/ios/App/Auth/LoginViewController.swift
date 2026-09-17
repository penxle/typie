import Auth
import Core
import Design
import UIKit

final class LoginViewController: UIViewController, UINavigationControllerDelegate {
  private let emailLogin: EmailLogin
  private let singleSignOn: SingleSignOnModel
  private var navigation: UINavigationController?
  private var login: UIViewController?
  private var emailModel: LoginModel?

  init(emailLogin: EmailLogin, singleSignOn: SingleSignOnModel) {
    self.emailLogin = emailLogin
    self.singleSignOn = singleSignOn
    super.init(nibName: nil, bundle: nil)
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    nil
  }

  override func viewDidLoad() {
    super.viewDidLoad()
    view.backgroundColor = .theme(\.surfaceCanvas)

    let login = ThemedHostingController.make(
      title: "",
      LoginScreen(singleSignOn: singleSignOn, onEmail: { [weak self] in self?.showEmailLogin() }))
    let navigation = ShellNavigationController.make(root: login)
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

  private func showEmailLogin() {
    guard let navigation, navigation.viewControllers.count == 1,
      navigation.transitionCoordinator == nil
    else { return }
    let model = LoginModel(
      login: { [emailLogin] in try await emailLogin(email: $0, password: $1) }, onSuccess: {})
    emailModel = model
    let screen = ThemedHostingController.make(title: "", EmailLoginScreen(model: model))
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
