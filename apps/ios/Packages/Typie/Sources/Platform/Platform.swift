#if canImport(UIKit)

  import Core
  import FactoryKit
  import GoogleSignIn
  import KakaoSDKAuth
  import KakaoSDKCommon
  import Logging
  import NidThirdPartyLogin
  import UIKit

  public enum Platform {
    @MainActor
    public static func register() {
      LoggingSystem.bootstrap { OSLogHandler(label: $0) }
      let deviceID = DeviceIdentity().id()
      Container.shared.device.register { DeviceInfo.current(id: deviceID) }
      Container.shared.singleSignOn.register {
        { provider, presenter in
          adapter(for: provider) { presenter() as? UIViewController }
        }
      }
      configure(Container.shared.appConfig())
      observeLifecycle(Container.shared.appLifecycle())
    }

    @MainActor
    static func configure(_ config: AppConfig) {
      KakaoSDK.initSDK(appKey: config.kakaoAppKey)
      NidOAuth.shared.initialize(
        appName: "타이피", clientId: config.naverClientID, clientSecret: config.naverClientSecret,
        urlScheme: "co.typie")
    }

    @MainActor
    static func observeLifecycle(_ lifecycle: AppLifecycle) {
      let center = NotificationCenter.default
      for name in [UIScene.willEnterForegroundNotification, UIScene.didActivateNotification] {
        _ = center.addObserver(forName: name, object: nil, queue: .main) { notification in
          let sender = notification.object.map { ObjectIdentifier($0 as AnyObject) }
          MainActor.assumeIsolated {
            guard isApplicationWindow(sender) else { return }
            lifecycle.update(foreground: true)
          }
        }
      }
      _ = center.addObserver(
        forName: UIScene.didEnterBackgroundNotification, object: nil, queue: .main
      ) { notification in
        let sender = notification.object.map { ObjectIdentifier($0 as AnyObject) }
        MainActor.assumeIsolated {
          guard isApplicationWindow(sender) else { return }
          let foreground = UIApplication.shared.connectedScenes.contains {
            ObjectIdentifier($0) != sender && $0.session.role == .windowApplication
              && ($0.activationState == .foregroundActive
                || $0.activationState == .foregroundInactive)
          }
          lifecycle.update(foreground: foreground)
        }
      }
    }

    @MainActor
    private static func isApplicationWindow(_ sender: ObjectIdentifier?) -> Bool {
      UIApplication.shared.connectedScenes.contains {
        ObjectIdentifier($0) == sender && $0.session.role == .windowApplication
      }
    }

    @MainActor
    public static func handle(_ url: URL) -> Bool {
      if GIDSignIn.sharedInstance.handle(url) { return true }
      if AuthController.handleOpenUrl(url: url) { return true }
      return NidOAuth.shared.handleURL(url)
    }

    @MainActor
    static func adapter(
      for provider: SingleSignOnProvider, presenter: @escaping @MainActor () -> UIViewController?
    ) -> any SingleSignOnAdapter {
      switch provider {
      case .google: GoogleSingleSignOn(presenter: presenter)
      case .kakao: KakaoSingleSignOn()
      case .naver: NaverSingleSignOn()
      case .apple: AppleSingleSignOn(anchor: { presenter()?.view.window })
      }
    }
  }

#endif
