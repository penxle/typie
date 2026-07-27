package co.typie.screen.document.document

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.domain.settings.SettingSwitch
import co.typie.editor.EditorOption
import co.typie.editor.EditorValues
import co.typie.editor.PagePresetCustom
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.form.FormState
import co.typie.graphql.TypieError
import co.typie.graphql.type.DocumentExportFormat
import co.typie.icons.Typie
import co.typie.platform.PlatformModule
import co.typie.platform.rememberShareAnchor
import co.typie.result.Result
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.AlertBanner
import co.typie.ui.component.Button
import co.typie.ui.component.SelectField
import co.typie.ui.component.SelectFieldItem
import co.typie.ui.component.Text
import co.typie.ui.component.editorsettings.EditorSettingsChipRow
import co.typie.ui.component.editorsettings.EditorSettingsTrailingChip
import co.typie.ui.component.editorsettings.MinPageSizeMm
import co.typie.ui.component.editorsettings.MmInputField
import co.typie.ui.component.editorsettings.clampPageMargins
import co.typie.ui.component.editorsettings.mmToPx
import co.typie.ui.component.editorsettings.pxToMm
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.Toast
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay

private class DocumentExportForm(scope: CoroutineScope) : FormState(scope) {
  val format = field(DocumentExportFormat.PDF) { focusable = false }
}

private val ExportFormatItems =
  listOf(
    SelectFieldItem(
      value = DocumentExportFormat.PDF,
      label = "PDF (Acrobat)",
      description = "인쇄와 공유에 적합한 고정 레이아웃",
      icon = Typie.FilePdf,
    ),
    SelectFieldItem(
      value = DocumentExportFormat.HWP,
      label = "HWP (한/글)",
      description = "편집 가능한 한컴오피스 호환 문서",
      icon = Typie.FileHwp,
    ),
    SelectFieldItem(
      value = DocumentExportFormat.DOCX,
      label = "DOCX (워드)",
      description = "편집 가능한 Microsoft Word 호환 문서",
      icon = Typie.FileDocx,
    ),
    SelectFieldItem(
      value = DocumentExportFormat.EPUB,
      label = "EPUB (전자책)",
      description = "전자책 리더에서 읽을 수 있는 표준 문서",
      icon = Typie.FileEpub,
    ),
  )

private fun formatNotice(format: DocumentExportFormat): String? =
  when (format) {
    DocumentExportFormat.HWP,
    DocumentExportFormat.DOCX -> "파일 특성상 일부 서식과 페이지 분할이 다르게 표시될 수 있어요."
    DocumentExportFormat.EPUB -> "전자책 특성상 문서에 포함된 장식 요소들이 간소화되고, 페이지 레이아웃이 적용되지 않아요."
    else -> null
  }

@Composable
context(_: SheetScope<Unit>)
internal fun DocumentExportSheet(
  model: DocumentViewModel,
  documentId: String,
  documentLayout: EditorDocumentLayoutSpec?,
) {
  val toast = LocalToast.current
  val focusManager = LocalFocusManager.current
  val scope = rememberCoroutineScope()
  val shareAnchor = rememberShareAnchor()
  val form = remember(scope) { DocumentExportForm(scope) }

  val canUseDocumentLayout = remember(documentLayout) { exportCanUseDocumentLayout(documentLayout) }
  var useDocumentLayout by remember(documentLayout) { mutableStateOf(canUseDocumentLayout) }
  var layout by remember(documentLayout) { mutableStateOf(exportInitialLayout(documentLayout)) }
  var customExpanded by remember { mutableStateOf(false) }
  var exporting by remember { mutableStateOf(false) }
  var elapsedSeconds by remember { mutableIntStateOf(0) }

  val format = form.format.value
  val controlsEnabled = exportLayoutControlsEnabled(format, useDocumentLayout) && !exporting
  val pageSizePreset = exportPageSizePreset(layout)
  val marginPreset = exportMarginPreset(layout)
  val marginItems =
    exportMarginOptions(layout).map { EditorOption(label = it.label, value = it.value) }

  LaunchedEffect(exporting) {
    elapsedSeconds = 0
    while (exporting) {
      delay(1_000)
      elapsedSeconds += 1
    }
  }

  suspend fun export() {
    focusManager.clearFocus()

    if (exporting) return

    exporting = true
    try {
      model
        .exportDocument(
          documentId = documentId,
          format = format,
          layout = exportLayoutInput(format, layout),
        )
        .withExportExceptionHandler(toast)
        .onOk { file ->
          val shared =
            PlatformModule.share.share(
              bytes = file.bytes,
              filename = file.filename,
              mimeType = file.mimeType,
              anchor = shareAnchor.value,
            )
          if (!shared) {
            toast.error("공유할 수 없어요.")
          }
        }
    } finally {
      exporting = false
    }
  }

  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = "파일로 내보내기",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    }
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(20.dp)) {
      Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("파일 형식", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

        SelectField(field = form.format, items = ExportFormatItems, enabled = !exporting)

        formatNotice(format)?.let { AlertBanner(text = it) }
      }

      if (canUseDocumentLayout) {
        Row(
          modifier = Modifier.fillMaxWidth(),
          horizontalArrangement = Arrangement.SpaceBetween,
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Text(
            "현재 페이지 설정 사용",
            style = AppTheme.typography.body,
            color = AppTheme.colors.textDefault,
          )
          SettingSwitch(
            checked = useDocumentLayout,
            onCheckedChange = {
              useDocumentLayout = it
              if (it) layout = exportInitialLayout(documentLayout)
            },
            enabled = format != DocumentExportFormat.EPUB && !exporting,
          )
        }
      }

      Column(
        modifier = Modifier.alpha(if (controlsEnabled) 1f else 0.4f),
        verticalArrangement = Arrangement.spacedBy(20.dp),
      ) {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
          Text("페이지 크기", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

          EditorSettingsChipRow(
            options =
              EditorValues.pageLayout.map {
                EditorOption(label = it.label.substringBefore(" "), value = it.value)
              },
            selected = pageSizePreset,
            onSelect = { preset ->
              if (!controlsEnabled) return@EditorSettingsChipRow
              layout = exportApplyPageSizePreset(layout, preset)
            },
            trailing = {
              EditorSettingsTrailingChip(
                label = "사용자 정의",
                selected = pageSizePreset == PagePresetCustom || customExpanded,
              ) {
                if (controlsEnabled) customExpanded = !customExpanded
              }
            },
          )
        }

        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
          Text("여백", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

          EditorSettingsChipRow(
            options = marginItems,
            selected = marginPreset,
            onSelect = { preset ->
              if (!controlsEnabled) return@EditorSettingsChipRow
              layout = exportApplyMarginPreset(layout, preset)
            },
            trailing = {
              EditorSettingsTrailingChip(
                label = "사용자 정의",
                selected = marginPreset == PagePresetCustom || customExpanded,
              ) {
                if (controlsEnabled) customExpanded = !customExpanded
              }
            },
          )
        }

        if (customExpanded && controlsEnabled) {
          Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(
              modifier = Modifier.fillMaxWidth(),
              horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
              MmInputField(
                label = "가로",
                valuePx = layout.pageWidth,
                onCommit = {
                  layout =
                    clampPageMargins(
                      layout.copy(pageWidth = mmToPx(maxOf(MinPageSizeMm, pxToMm(it))))
                    )
                },
                modifier = Modifier.weight(1f),
              )
              MmInputField(
                label = "세로",
                valuePx = layout.pageHeight,
                onCommit = {
                  layout =
                    clampPageMargins(
                      layout.copy(pageHeight = mmToPx(maxOf(MinPageSizeMm, pxToMm(it))))
                    )
                },
                modifier = Modifier.weight(1f),
              )
            }

            Row(
              modifier = Modifier.fillMaxWidth(),
              horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
              MmInputField(
                label = "상",
                valuePx = layout.pageMarginTop,
                onCommit = { layout = clampPageMargins(layout.copy(pageMarginTop = it)) },
                modifier = Modifier.weight(1f),
              )
              MmInputField(
                label = "하",
                valuePx = layout.pageMarginBottom,
                onCommit = { layout = clampPageMargins(layout.copy(pageMarginBottom = it)) },
                modifier = Modifier.weight(1f),
              )
            }

            Row(
              modifier = Modifier.fillMaxWidth(),
              horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
              MmInputField(
                label = "좌",
                valuePx = layout.pageMarginLeft,
                onCommit = { layout = clampPageMargins(layout.copy(pageMarginLeft = it)) },
                modifier = Modifier.weight(1f),
              )
              MmInputField(
                label = "우",
                valuePx = layout.pageMarginRight,
                onCommit = { layout = clampPageMargins(layout.copy(pageMarginRight = it)) },
                modifier = Modifier.weight(1f),
              )
            }
          }
        }
      }

      exportProgressNotice(elapsedSeconds)
        ?.takeIf { exporting }
        ?.let { Text(it, style = AppTheme.typography.caption, color = AppTheme.colors.textMuted) }

      Button(
        text = "내보내기",
        onClick = { export() },
        loading = exporting,
        loadingText = "내보내는 중",
        modifier = shareAnchor.modifier,
      )
    }
  }
}

private fun <T, E> Result<T, E>.withExportExceptionHandler(toast: Toast): Result<T, E> {
  val exception = (this as? Result.Exception)?.exception
  if (exception is TypieError && exception.code == "document_projection_degraded") {
    toast.error(exception.message ?: "문서를 내보낼 수 없는 상태예요.")
    return this
  }

  return withDefaultExceptionHandler(toast)
}
