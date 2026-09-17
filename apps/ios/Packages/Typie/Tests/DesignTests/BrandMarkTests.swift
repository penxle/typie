import Foundation
import Testing

#if canImport(AppKit)
  import AppKit
#endif

@testable import Design

@Suite struct BrandMarkTests {
  @Test func marksAreOrderedAsOnTheLoginSheet() {
    #expect(TBrandMark.allCases == [.google, .kakao, .naver, .apple])
  }

  #if canImport(AppKit)
    @Test func everyMarkResolvesToABundledAsset() {
      for mark in TBrandMark.allCases {
        #expect(Bundle.module.image(forResource: mark.assetName) != nil, "\(mark.rawValue)")
      }
    }
  #endif
}
