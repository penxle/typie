import Foundation
import FirebaseMessaging
import UserNotifications
import UIKit

@objcMembers public final class PushNotificationPayload: NSObject {
  public let title: String?
  public let body: String?
  public let data: [String: String]

  public init(title: String?, body: String?, data: [String: String]) {
    self.title = title
    self.body = body
    self.data = data
  }
}

@objcMembers public final class PushNotificationBridge: NSObject,
                                                        MessagingDelegate,
                                                        UNUserNotificationCenterDelegate {
  public var onToken: ((String) -> Void)?
  public var onMessage: ((PushNotificationPayload) -> Void)?

  public override init() {
    super.init()
  }

  public func attach(to application: UIApplication) {
    UNUserNotificationCenter.current().delegate = self
    Messaging.messaging().delegate = self
    application.registerForRemoteNotifications()
  }

  public func requestAuthorization(completion: @escaping @Sendable (Bool) -> Void) {
    UNUserNotificationCenter.current().requestAuthorization(
      options: [.alert, .badge, .sound]
    ) { granted, _ in
      DispatchQueue.main.async { completion(granted) }
    }
  }

  public func fetchToken(completion: @escaping @Sendable (String?) -> Void) {
    Messaging.messaging().token { token, _ in
      completion(token)
    }
  }

  public func deleteToken(completion: @escaping @Sendable () -> Void) {
    Messaging.messaging().deleteToken { _ in
      completion()
    }
  }

  // MARK: - MessagingDelegate

  public func messaging(_ messaging: Messaging, didReceiveRegistrationToken fcmToken: String?) {
    if let token = fcmToken {
      onToken?(token)
    }
  }

  // MARK: - UNUserNotificationCenterDelegate

  public func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    willPresent notification: UNNotification,
    withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
  ) {
    let content = notification.request.content
    var data: [String: String] = [:]
    for (key, value) in content.userInfo {
      if let k = key as? String {
        data[k] = "\(value)"
      }
    }
    let payload = PushNotificationPayload(
      title: content.title.isEmpty ? nil : content.title,
      body: content.body.isEmpty ? nil : content.body,
      data: data
    )
    onMessage?(payload)
    completionHandler([])
  }
}
