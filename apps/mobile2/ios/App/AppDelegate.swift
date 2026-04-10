import Compose
import KakaoSDKCommon
import NidThirdPartyLogin
import UIKit

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
  func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
  ) -> Bool {
    InitLoggerKt.doInitLogger()

    let info = Bundle.main.infoDictionary ?? [:]

    if let kakaoAppKey = info["KAKAO_NATIVE_APP_KEY"] as? String {
      KakaoSDK.initSDK(appKey: kakaoAppKey)
    }

    if
      let naverClientId = info["NAVER_CLIENT_ID"] as? String,
      let naverClientSecret = info["NAVER_CLIENT_SECRET"] as? String
    {
      NidOAuth.shared.initialize(
        appName: "타이피",
        clientId: naverClientId,
        clientSecret: naverClientSecret,
        urlScheme: "co.typie"
      )
    }

    return true
  }

  func application(
    _ application: UIApplication,
    configurationForConnecting connectingSceneSession: UISceneSession,
    options: UIScene.ConnectionOptions
  ) -> UISceneConfiguration {
    let configuration = UISceneConfiguration(
      name: "Default Configuration",
      sessionRole: connectingSceneSession.role
    )
    configuration.delegateClass = SceneDelegate.self
    return configuration
  }
}
