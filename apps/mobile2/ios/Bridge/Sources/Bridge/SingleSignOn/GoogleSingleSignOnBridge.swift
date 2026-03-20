import Foundation
import GoogleSignIn
import UIKit

@MainActor @objcMembers public class GoogleSingleSignOnBridge : NSObject {
  public func authenticate(completion: @escaping @Sendable (String?, NSError?) -> Void) {
    guard let rootViewController = UIApplication.shared.presentingViewController else {
      completion(nil, NSError(
        domain: "co.typie.ios.bridge",
        code: -1,
        userInfo: [NSLocalizedDescriptionKey: "No root view controller"],
      ))
      return
    }

    GIDSignIn.sharedInstance.signIn(withPresenting: rootViewController) { result, error in
      if let error = error {
        completion(nil, error as NSError)
      } else {
        completion(result?.serverAuthCode, nil)
      }
    }
  }
}
