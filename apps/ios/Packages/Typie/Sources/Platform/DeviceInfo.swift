#if canImport(UIKit)

  import Core
  import UIKit

  extension DeviceInfo {
    @MainActor
    static func current(id: String) -> DeviceInfo {
      DeviceInfo(id: id, model: UIDevice.current.model, systemName: UIDevice.current.systemName)
    }
  }

#endif
