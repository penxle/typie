#if canImport(UIKit)

  import Core
  import FactoryKit
  import GoogleSignIn
  import KakaoSDKAuth
  import KakaoSDKCommon
  import NidThirdPartyLogin
  import UIKit

  public enum Platform {
    @MainActor
    public static func register() {
      let deviceID = DeviceIdentity().id()
      Container.shared.device.register { DeviceInfo.current(id: deviceID) }
      Container.shared.singleSignOn.register {
        { provider, presenter in
          adapter(for: provider) { presenter() as? UIViewController }
        }
      }
      configure(Container.shared.appConfig())
    }

    @MainActor
    static func configure(_ config: AppConfig) {
      KakaoSDK.initSDK(appKey: config.kakaoAppKey)
      NidOAuth.shared.initialize(
        appName: "타이피", clientId: config.naverClientID, clientSecret: config.naverClientSecret,
        urlScheme: "co.typie")
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
