import Design
import Editor

struct DebugOverlayToggle: Sendable {
  static let all = [
    DebugOverlayToggle(overlay: .viewportGuides, label: "뷰포트 기준선", icon: LucideIcon.panelTop),
    DebugOverlayToggle(overlay: .bodyAreas, label: "바디 영역", icon: LucideIcon.panelBottom),
    DebugOverlayToggle(
      overlay: .pageSurfaces, label: "페이지 표면", icon: LucideIcon.inspectionPanel),
  ]

  let overlay: EditorDebugOverlays
  let label: String
  let icon: TIconName

  func title(in overlays: EditorDebugOverlays) -> String {
    "\(label) \(overlays.contains(overlay) ? "끄기" : "켜기")"
  }
}
