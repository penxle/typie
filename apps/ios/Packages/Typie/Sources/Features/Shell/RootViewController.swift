#if canImport(UIKit)

  import Core
  import Design
  import FactoryKit
  import SwiftUI
  import UIKit

  public final class RootViewController: UIViewController {
    private enum Stage {
      case launching
      case auth
      case main
    }

    private static let transitionDuration: TimeInterval = 0.2

    private let authState = Container.shared.authState()
    private let authService = Container.shared.authService()
    private var stage: Stage?

    public init() {
      super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    public required init?(coder: NSCoder) {
      nil
    }

    override public func viewDidLoad() {
      super.viewDidLoad()
      view.backgroundColor = .theme(\.surfaceCanvas)
      show(.launching)

      Task { [weak self, authService] in
        try? await authService.renew()
        guard let self else { return }
        let authState = authState
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
        return ThemedHostingController(title: "", LaunchingScreen())
      case .auth:
        Container.shared.manager.reset(scope: .session)
        return AuthFlowController()
      case .main:
        Container.shared.liveUpdates().start()
        let router = Container.shared.router()
        let controller = ShellController(
          tabRoot: router.tabRoot(),
          openSearchHit: { hit, presenter in router.pushSearchHit(hit, from: presenter) },
          createItems: { top in router.createItems(for: top) })
        Container.shared.sites().onCreated = { [weak self] in self?.dismiss(animated: true) }
        return controller
      }
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

#endif
