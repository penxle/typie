import Core
import Design
import UIKit

@MainActor
final class AppEnvironment {
  let config: AppConfig?
  let services: CoreServices?
  let toast = TToastCenter()
  let dialog = TDialogCenter()

  init() {
    guard
      let config = try? AppConfig.load(
        oidcClientSecret: Secrets.oidcClientSecret, naverClientSecret: Secrets.naverClientSecret)
    else {
      self.config = nil
      services = nil
      return
    }
    self.config = config
    let deviceID = DeviceIdentity().id()
    let model = UIDevice.current.model
    let systemName = UIDevice.current.systemName
    services = CoreAssembly.make(config: config, deviceID: deviceID) {
      DeviceHeaders.make(deviceID: deviceID, model: model, systemName: systemName)
    }
  }
}
