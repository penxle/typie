import Foundation
import NidThirdPartyLogin

@MainActor @objcMembers public class NaverSingleSignOnBridge: NSObject {
    public func authenticate(completion: @escaping @Sendable (String?, NSError?) -> Void) {
        NidOAuth.shared.requestLogin { result in
            switch result {
            case .success(let result):
                completion(result.accessToken.tokenString, nil)
            case .failure(let error):
                completion(nil, error as NSError)
            }
        }
    }
}
