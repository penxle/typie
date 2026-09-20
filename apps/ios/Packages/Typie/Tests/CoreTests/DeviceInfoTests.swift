import Testing

@testable import Core

@Suite struct DeviceInfoTests {
  @Test func buildsThreeHeadersWithUppercasedPlatform() {
    let headers = DeviceInfo(id: "abc", model: "iPhone", systemName: "iOS").headers
    #expect(
      headers == ["X-Device-Id": "abc", "X-Device-Name": "iPhone", "X-Device-Platform": "IOS"])
  }
}
