import Foundation
import KakaoSDKUser

@MainActor @objcMembers public class KakaoSingleSignOnBridge: NSObject {
  public func authenticate(
    completion: @escaping @Sendable (String?, NSError?) -> Void
  ) {
    UserApi.shared.loginWithKakaoAccount(prompts: [.SelectAccount]) {
      token,
      error in
      if let error = error {
        completion(nil, error as NSError)
      } else {
        completion(token?.accessToken, nil)
      }
    }
  }
}
