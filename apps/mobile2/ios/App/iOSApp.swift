import Compose
import GoogleSignIn
import KakaoSDKAuth
import KakaoSDKCommon
import NidThirdPartyLogin
import SwiftUI

@main
struct iOSApp: App {
  init() {
    KoinInitKt.doInitKoin()

    let info = Bundle.main.infoDictionary!

    KakaoSDK.initSDK(appKey: info["KAKAO_NATIVE_APP_KEY"] as! String)

    NidOAuth.shared.initialize(
      appName: "타이피",
      clientId: info["NAVER_CLIENT_ID"] as! String,
      clientSecret: info["NAVER_CLIENT_SECRET"] as! String,
      urlScheme: "co.typie"
    )
  }

  var body: some Scene {
    WindowGroup {
      ContentView()
        .onOpenURL { url in
          if GIDSignIn.sharedInstance.handle(url) {
            return
          }

          if AuthController.handleOpenUrl(url: url) {
            return
          }
          
          if NidOAuth.shared.handleURL(url) {
            return
          }
        }
    }
  }
}
