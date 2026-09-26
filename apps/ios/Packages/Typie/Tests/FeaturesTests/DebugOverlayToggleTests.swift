import Design
import Editor
import Testing

@testable import Features

@Suite struct DebugOverlayToggleTests {
  @Test func offersTheThreeOverlaysInToolMenuOrder() {
    #expect(DebugOverlayToggle.all.map(\.overlay) == [.viewportGuides, .bodyAreas, .pageSurfaces])
    #expect(
      DebugOverlayToggle.all.map(\.icon) == [
        LucideIcon.panelTop, LucideIcon.panelBottom, LucideIcon.inspectionPanel,
      ])
  }

  @Test func offersToTurnOnAnOverlayThatIsOff() {
    #expect(
      DebugOverlayToggle.all.map { $0.title(in: []) } == [
        "뷰포트 기준선 켜기", "바디 영역 켜기", "페이지 표면 켜기",
      ])
  }

  @Test func offersToTurnOffAnOverlayThatIsOn() {
    #expect(
      DebugOverlayToggle.all.map { $0.title(in: [.bodyAreas]) } == [
        "뷰포트 기준선 켜기", "바디 영역 끄기", "페이지 표면 켜기",
      ])
  }
}
