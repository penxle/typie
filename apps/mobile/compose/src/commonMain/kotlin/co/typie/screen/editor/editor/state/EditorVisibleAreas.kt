package co.typie.screen.editor.editor.state

import co.typie.editor.scroll.EditorVisibleArea

internal data class EditorOverlayOcclusion(
  val top: Float = 0f,
  val bottom: Float = 0f,
  val bottomScrollReserve: Float = bottom,
)

internal data class EditorVisibleAreas(
  // 추가 오버레이를 반영하기 전의 영역입니다. 기준 영역이 바뀌면 위치가 다시 흔들릴 수 있는
  // UI를 배치할 때 사용합니다.
  val base: EditorVisibleArea,
  // 실제 에디터 내용, 커서, 자동 스크롤이 따르는 영역입니다.
  // 위/아래 오버레이가 차지한 공간을 제외합니다.
  val editor: EditorVisibleArea,
  // 문서 끝까지 스크롤할 때 필요한 하단 여유 공간 계산에 사용하는 영역입니다.
  // 화면에 보이는 오버레이 높이와 스크롤 여유로 확보해야 하는 높이가 다를 수 있습니다.
  val bottomSpacer: EditorVisibleArea,
)

internal fun EditorScreenState.resolveEditorVisibleAreas(
  topInset: Float,
  rawBottomSafeInset: Float,
  rawEditorInputBottomInset: Float,
  rawSubPaneBottomInset: Float = 0f,
  overlayOcclusion: EditorOverlayOcclusion = EditorOverlayOcclusion(),
): EditorVisibleAreas {
  val base =
    resolveVisibleArea(
      topInset = topInset,
      rawBottomSafeInset = rawBottomSafeInset,
      rawEditorInputBottomInset = rawEditorInputBottomInset,
      rawSubPaneBottomInset = rawSubPaneBottomInset,
    )
  val editor =
    resolveVisibleArea(
      topInset = topInset + overlayOcclusion.top,
      rawBottomSafeInset = rawBottomSafeInset,
      rawEditorInputBottomInset = rawEditorInputBottomInset + overlayOcclusion.bottom,
      rawSubPaneBottomInset = rawSubPaneBottomInset,
    )
  val bottomSpacer =
    resolveVisibleArea(
      topInset = topInset + overlayOcclusion.top,
      rawBottomSafeInset = rawBottomSafeInset,
      rawEditorInputBottomInset = rawEditorInputBottomInset + overlayOcclusion.bottomScrollReserve,
      rawSubPaneBottomInset = rawSubPaneBottomInset,
    )

  return EditorVisibleAreas(base = base, editor = editor, bottomSpacer = bottomSpacer)
}
