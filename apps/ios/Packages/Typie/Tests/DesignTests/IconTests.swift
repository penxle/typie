import Foundation
import Testing

#if canImport(AppKit)
  import AppKit
#endif

@testable import Design

@Suite struct IconTests {
  @Test func generatedNamesPointAtNamespacedAssets() {
    #expect(LucideIcon.aArrowDown.assetName == "lucide/a-arrow-down")
    #expect(LucideIcon.construction.assetName == "lucide/construction")
    #expect(TypieIcon.bellFilled.assetName == "typie/bell-filled")
  }

  @Test func catalogsAreBundled() {
    #expect(Bundle.module.url(forResource: "Assets", withExtension: "car") != nil)
  }

  #if canImport(AppKit)
    @Test func namespacedAssetsResolve() {
      #expect(Bundle.module.image(forResource: LucideIcon.aArrowDown.assetName) != nil)
      #expect(Bundle.module.image(forResource: TypieIcon.bellFilled.assetName) != nil)
      #expect(Bundle.module.image(forResource: "a-arrow-down") == nil)
      #expect(Bundle.module.image(forResource: "logo-full") != nil)
    }
  #endif
}
