import FactoryKit
import FactoryTesting
import Testing

@testable import Features

@MainActor
@Suite(.container) struct FeaturesContainerTests {
  @Test func editorResourcesAndFontLoaderOutliveTheSession() {
    let resources = Container.shared.editorResources()
    let fonts = Container.shared.fontLoader()
    Container.shared.manager.reset(scope: .session)
    #expect(Container.shared.editorResources() == resources)
    #expect(Container.shared.fontLoader() == fonts)
  }

  @Test func fontLoaderIsReadyOnceTheResourcesLoad() async throws {
    _ = try await Container.shared.editorResources().value
    _ = try await Container.shared.fontLoader().value
  }
}
