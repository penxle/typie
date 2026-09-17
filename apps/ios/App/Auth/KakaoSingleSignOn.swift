import Core
import KakaoSDKAuth
import KakaoSDKCommon
import KakaoSDKUser

@MainActor
final class KakaoSingleSignOn: SingleSignOnAdapter {
  func authenticate() async throws -> SingleSignOnCredential {
    let accessToken: String? = try await withCheckedThrowingContinuation { continuation in
      let completion: (OAuthToken?, (any Error)?) -> Void = { token, error in
        if let error {
          continuation.resume(throwing: Self.mapped(error))
        } else {
          continuation.resume(returning: token?.accessToken)
        }
      }
      if UserApi.isKakaoTalkLoginAvailable() {
        UserApi.shared.loginWithKakaoTalk(completion: completion)
      } else {
        UserApi.shared.loginWithKakaoAccount(prompts: [.SelectAccount], completion: completion)
      }
    }
    guard let accessToken else { throw SingleSignOnError.missingCredential }
    return SingleSignOnCredential(provider: .kakao, params: ["access_token": accessToken])
  }

  private static func mapped(_ error: any Error) -> any Error {
    if let error = error as? SdkError, error.isClientFailed,
      error.getClientError().reason == .Cancelled
    {
      return SingleSignOnError.cancelled
    }
    return error
  }
}
