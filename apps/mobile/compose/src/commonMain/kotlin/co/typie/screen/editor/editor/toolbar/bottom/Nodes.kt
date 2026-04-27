package co.typie.screen.editor.editor.toolbar.bottom

import androidx.compose.runtime.Composable
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton

@Composable
internal fun BottomPanelNodes() {
  EditorToolbarButton(icon = Lucide.Image, contentDescription = "이미지", onClick = {})
  EditorToolbarButton(icon = Lucide.Paperclip, contentDescription = "파일", onClick = {})
  EditorToolbarButton(icon = Lucide.FileUp, contentDescription = "임베드", onClick = {})
  EditorToolbarButton(icon = Lucide.Minus, contentDescription = "구분선", onClick = {})
  EditorToolbarButton(icon = Lucide.Quote, contentDescription = "인용구", onClick = {})
  EditorToolbarButton(icon = Lucide.GalleryVerticalEnd, contentDescription = "강조", onClick = {})
  EditorToolbarButton(icon = Lucide.ChevronsDownUp, contentDescription = "접기", onClick = {})
  EditorToolbarButton(icon = Lucide.Table, contentDescription = "표", onClick = {})
  EditorToolbarButton(icon = Lucide.List, contentDescription = "목록", onClick = {})
  EditorToolbarButton(icon = Lucide.PanelTopDashed, contentDescription = "페이지 나누기", onClick = {})
  EditorToolbarButton(icon = Lucide.CornerDownLeft, contentDescription = "하드 브레이크", onClick = {})
}
