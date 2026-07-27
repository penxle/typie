package co.typie.screen.document.document

import co.typie.editor.EditorPageMarginOption
import co.typie.editor.EditorValues
import co.typie.editor.applyMarginPreset
import co.typie.editor.applyPageSizePreset
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.pageMarginOptionsOf
import co.typie.editor.pageMarginPresetOf
import co.typie.editor.pageSizePresetOf
import co.typie.graphql.type.DocumentExportFormat
import co.typie.graphql.type.ExportDocumentPageLayoutInput
import co.typie.ui.component.editorsettings.clampPageMargins
import kotlin.math.roundToInt

private const val ExportSlowNoticeSeconds = 15
private const val ExportVerySlowNoticeSeconds = 60

private val DefaultExportPageLayout = EditorValues.pageLayout.first { it.value == "a4" }.layout

internal fun exportInitialLayout(spec: EditorDocumentLayoutSpec?): LayoutMode.Paginated =
  when (spec) {
    is EditorDocumentLayoutSpec.Paginated ->
      LayoutMode.Paginated(
        pageWidth = spec.pageWidth.roundToInt(),
        pageHeight = spec.pageHeight.roundToInt(),
        pageMarginTop = spec.pageMarginTop.roundToInt(),
        pageMarginBottom = spec.pageMarginBottom.roundToInt(),
        pageMarginLeft = spec.pageMarginLeft.roundToInt(),
        pageMarginRight = spec.pageMarginRight.roundToInt(),
      )
    else -> DefaultExportPageLayout
  }

internal fun exportCanUseDocumentLayout(spec: EditorDocumentLayoutSpec?): Boolean =
  spec is EditorDocumentLayoutSpec.Paginated

internal fun exportMarginOptions(layout: LayoutMode.Paginated): List<EditorPageMarginOption> =
  pageMarginOptionsOf(layout, fallback = emptyList())

internal fun exportPageSizePreset(layout: LayoutMode.Paginated): String = pageSizePresetOf(layout)

internal fun exportMarginPreset(layout: LayoutMode.Paginated): String =
  pageMarginPresetOf(layout, exportMarginOptions(layout))

internal fun exportApplyPageSizePreset(
  layout: LayoutMode.Paginated,
  preset: String,
): LayoutMode.Paginated =
  clampPageMargins(applyPageSizePreset(layout, preset, exportMarginOptions(layout)))

internal fun exportApplyMarginPreset(
  layout: LayoutMode.Paginated,
  preset: String,
): LayoutMode.Paginated =
  clampPageMargins(applyMarginPreset(layout, preset, exportMarginOptions(layout)))

internal fun exportLayoutInput(
  format: DocumentExportFormat,
  layout: LayoutMode.Paginated,
): ExportDocumentPageLayoutInput? {
  if (format == DocumentExportFormat.EPUB) return null

  return ExportDocumentPageLayoutInput(
    pageWidth = layout.pageWidth,
    pageHeight = layout.pageHeight,
    pageMarginTop = layout.pageMarginTop,
    pageMarginBottom = layout.pageMarginBottom,
    pageMarginLeft = layout.pageMarginLeft,
    pageMarginRight = layout.pageMarginRight,
  )
}

internal fun exportLayoutControlsEnabled(
  format: DocumentExportFormat,
  useDocumentLayout: Boolean,
): Boolean = format != DocumentExportFormat.EPUB && !useDocumentLayout

internal fun exportProgressNotice(elapsedSeconds: Int): String? =
  when {
    elapsedSeconds >= ExportVerySlowNoticeSeconds -> "계속 진행 중이에요. 앱을 닫지 말아 주세요."
    elapsedSeconds >= ExportSlowNoticeSeconds -> "문서가 길면 1분 이상 걸릴 수 있어요."
    else -> null
  }
