import FactoryKit
import FactoryTesting
import Testing

@testable import Design

@MainActor
@Suite(.container) struct ContainerTests {
  @Test func centersAndThemeResolveOnce() {
    let toast = Container.shared.toast()
    let dialog = Container.shared.dialog()
    let theme = Container.shared.theme()

    #expect(Container.shared.toast() === toast)
    #expect(Container.shared.dialog() === dialog)
    #expect(Container.shared.theme() === theme)
  }
}
