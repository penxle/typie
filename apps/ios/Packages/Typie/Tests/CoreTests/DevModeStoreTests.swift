import Observation
import Testing

@testable import Core

@MainActor @Suite struct DevModeStoreTests {
  @Test func startsDisabledWhenNothingIsStored() {
    withFreshDefaults { defaults in
      #expect(DevModeStore(defaults: defaults).isEnabled == false)
    }
  }

  @Test func startsFromTheStoredValue() {
    withFreshDefaults { defaults in
      defaults.set(true, forKey: "dev_mode")
      #expect(DevModeStore(defaults: defaults).isEnabled)
    }
  }

  @Test func enablingPersistsUnderTheDeviceGlobalKey() {
    withFreshDefaults { defaults in
      let store = DevModeStore(defaults: defaults)
      store.setEnabled(true)
      #expect(store.isEnabled)
      #expect(defaults.object(forKey: "dev_mode") as? Bool == true)
      #expect(DevModeStore(defaults: defaults).isEnabled)
    }
  }

  @Test func disablingPersists() {
    withFreshDefaults { defaults in
      defaults.set(true, forKey: "dev_mode")
      let store = DevModeStore(defaults: defaults)
      store.setEnabled(false)
      #expect(store.isEnabled == false)
      #expect(defaults.object(forKey: "dev_mode") as? Bool == false)
      #expect(DevModeStore(defaults: defaults).isEnabled == false)
    }
  }

  @Test func changesNotifyObservers() {
    withFreshDefaults { defaults in
      let store = DevModeStore(defaults: defaults)
      let recorder = CallRecorder()
      withObservationTracking {
        _ = store.isEnabled
      } onChange: {
        recorder.record("isEnabled")
      }
      store.setEnabled(true)
      #expect(recorder.calls == ["isEnabled"])
    }
  }
}
