#if canImport(UIKit)

  import UIKit

  @MainActor
  final class EntityCreationContext {
    let siteId: () -> String?
    let parentEntityId: String?
    let didCreate: () -> Void

    init(siteId: @escaping () -> String?, parentEntityId: String?, didCreate: @escaping () -> Void)
    {
      self.siteId = siteId
      self.parentEntityId = parentEntityId
      self.didCreate = didCreate
    }
  }

  private nonisolated(unsafe) var creationContextKey: UInt8 = 0

  extension UIViewController {
    var creationContext: EntityCreationContext? {
      get { objc_getAssociatedObject(self, &creationContextKey) as? EntityCreationContext }
      set {
        objc_setAssociatedObject(
          self, &creationContextKey, newValue, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
      }
    }
  }

#endif
