package co.typie.editor.external

import androidx.compose.runtime.Composable
import co.typie.editor.ffi.ExternalElementData
import co.typie.icons.Lucide
import co.typie.ui.icon.IconData

@Composable
context(scope: EditorExternalElementRenderScope)
internal fun EditorGenericExternalElement(data: ExternalElementData) {
  val content = data.content()
  EditorExternalElementPlaceholder(icon = content.icon, text = content.label)
}

private data class ExternalElementContent(val icon: IconData, val label: String)

private fun ExternalElementData.content(): ExternalElementContent =
  when (this) {
    is ExternalElementData.Image -> ExternalElementContent(Lucide.Image, "이미지")
    is ExternalElementData.File -> ExternalElementContent(Lucide.File, "파일")
    is ExternalElementData.Embed ->
      ExternalElementContent(Lucide.FileUp, "링크 임베드(Youtube, Google Drive, 일반 링크 등)")
    is ExternalElementData.Archived -> ExternalElementContent(Lucide.Archive, "보관된 블록")
  }
