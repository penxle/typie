import Auth
import Core
import Design
import SwiftUI
import UIKit

final class RootViewController: UIViewController {
  private enum Stage {
    case launching
    case unavailable
    case auth
    case main
  }

  private static let transitionDuration: TimeInterval = 0.2

  private let environment: AppEnvironment
  private let theme: ThemeSettings
  private var stage: Stage?

  init(environment: AppEnvironment, theme: ThemeSettings) {
    self.environment = environment
    self.theme = theme
    super.init(nibName: nil, bundle: nil)
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    nil
  }

  override func viewDidLoad() {
    super.viewDidLoad()
    view.backgroundColor = .theme(\.surfaceCanvas)
    show(.launching)

    guard let services = environment.services else {
      show(.unavailable)
      return
    }

    Task { [weak self] in
      try? await services.authService.renew()
      guard let self else { return }
      let authState = services.authState
      keepObserving(while: self) { [weak self] in
        switch authState.state {
        case .authenticated: self?.show(.main)
        case .unauthenticated: self?.show(.auth)
        }
      }
    }
  }

  private func show(_ stage: Stage) {
    guard self.stage != stage else { return }
    let child = makeChild(for: stage)
    self.stage = stage
    guard presentedViewController != nil else {
      setChild(child)
      return
    }
    dismiss(animated: true) { [weak self] in self?.setChild(child) }
  }

  private func makeChild(for stage: Stage) -> UIViewController {
    switch stage {
    case .launching:
      return ThemedHostingController.make(title: "", LaunchingScreen())
    case .unavailable:
      return ThemedHostingController.make(title: "", ConfigurationMissingScreen())
    case .auth, .main:
      guard let services = environment.services else {
        return ThemedHostingController.make(title: "", ConfigurationMissingScreen())
      }
      if stage == .auth {
        let singleSignOn = SingleSignOnModel(
          login: { [weak self] provider in
            let adapter = await SingleSignOnSDK.adapter(for: provider) { [weak self] in
              self?.topPresenter
            }
            let credential = try await adapter.authenticate()
            try await services.singleSignOnLogin(credential)
          }, toast: environment.toast, onSuccess: {})
        return LoginViewController(emailLogin: services.emailLogin, singleSignOn: singleSignOn)
      }
      let router = Router(theme: theme, services: services)
      return MainTabBarController(
        rootProvider: { tab in router.root(for: tab) },
        createAction: { tab in router.createAction(for: tab) })
    }
  }

  private var topPresenter: UIViewController {
    var controller: UIViewController = self
    while let presented = controller.presentedViewController {
      controller = presented
    }
    return controller
  }

  private func setChild(_ controller: UIViewController) {
    guard let previous = children.first else {
      embed(controller)
      return
    }
    addChild(controller)
    controller.view.frame = view.bounds
    controller.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
    previous.willMove(toParent: nil)
    let duration = UIAccessibility.isReduceMotionEnabled ? 0 : Self.transitionDuration
    transition(
      from: previous, to: controller, duration: duration, options: .transitionCrossDissolve,
      animations: nil
    ) { _ in
      previous.removeFromParent()
      controller.didMove(toParent: self)
    }
  }
}

private struct LaunchingScreen: View {
  var body: some View {
    TLogo(height: TLogo.launchHeight)
      .frame(maxWidth: .infinity, maxHeight: .infinity)
      .ignoresSafeArea()
      .canvasBackground()
  }
}

private struct ConfigurationMissingScreen: View {
  var body: some View {
    TText("AppConfig missing: run just env", style: TTypography.body)
      .padding(16)
      .frame(maxWidth: .infinity, maxHeight: .infinity)
      .canvasBackground()
  }
}
