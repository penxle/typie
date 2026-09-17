import Core
import GoogleSignIn
import KakaoSDKAuth
import KakaoSDKCommon
import NidThirdPartyLogin
import UIKit

enum SingleSignOnSDK {
  @MainActor
  static func configure(_ config: AppConfig) {
    KakaoSDK.initSDK(appKey: config.kakaoAppKey)
    NidOAuth.shared.initialize(
      appName: "타이피", clientId: config.naverClientID, clientSecret: config.naverClientSecret,
      urlScheme: "co.typie")
  }

  @MainActor
  static func handle(_ url: URL) -> Bool {
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
