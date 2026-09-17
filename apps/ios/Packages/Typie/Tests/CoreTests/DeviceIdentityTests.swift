import Foundation
import Testing

@testable import Core

@Suite struct DeviceIdentityTests {
  @Test func generatesLowercaseHexOf32Characters() {
    withFreshDefaults { defaults in
      let id = DeviceIdentity(defaults: defaults).id()
      #expect(id.count == 32)
      #expect(id.allSatisfy { $0.isHexDigit && !$0.isUppercase })
    }
  }

  @Test func persistsAcrossInstances() {
    withFreshDefaults { defaults in
      #expect(DeviceIdentity(defaults: defaults).id() == DeviceIdentity(defaults: defaults).id())
    }
  }

  @Test func reusesStoredValue() {
    withFreshDefaults { defaults in
      defaults.set("0123456789abcdef0123456789abcdef", forKey: "device_id")
      #expect(DeviceIdentity(defaults: defaults).id() == "0123456789abcdef0123456789abcdef")
    }
  }
}
