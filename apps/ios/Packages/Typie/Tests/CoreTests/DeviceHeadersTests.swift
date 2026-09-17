import Testing

@testable import Core

@Suite struct DeviceHeadersTests {
  @Test func buildsThreeHeadersWithUppercasedPlatform() {
    let headers = DeviceHeaders.make(deviceID: "abc", model: "iPhone", systemName: "iOS")
    #expect(
      headers == ["X-Device-Id": "abc", "X-Device-Name": "iPhone", "X-Device-Platform": "IOS"])
  }
}
