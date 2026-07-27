package co.typie.editor

import co.typie.editor.ffi.LayoutMode

internal const val PagePresetCustom = "custom"

internal fun pageLayoutOptionOf(layout: LayoutMode.Paginated): EditorPageLayoutOption? =
  EditorValues.pageLayout.firstOrNull {
    it.layout.pageWidth == layout.pageWidth && it.layout.pageHeight == layout.pageHeight
  }

internal fun pageSizePresetOf(layout: LayoutMode.Paginated): String =
  pageLayoutOptionOf(layout)?.value ?: PagePresetCustom

internal fun pageMarginOptionsOf(
  layout: LayoutMode.Paginated,
  fallback: List<EditorPageMarginOption>,
): List<EditorPageMarginOption> = pageLayoutOptionOf(layout)?.margins ?: fallback

internal fun pageMarginPresetOf(
  layout: LayoutMode.Paginated,
  options: List<EditorPageMarginOption>,
): String =
  options
    .firstOrNull {
      layout.pageMarginTop == it.top &&
        layout.pageMarginBottom == it.bottom &&
        layout.pageMarginLeft == it.left &&
        layout.pageMarginRight == it.right
    }
    ?.value ?: PagePresetCustom

internal fun applyMarginPreset(
  layout: LayoutMode.Paginated,
  preset: String,
  options: List<EditorPageMarginOption>,
): LayoutMode.Paginated {
  val margin = options.firstOrNull { it.value == preset } ?: return layout

  return layout.copy(
    pageMarginTop = margin.top,
    pageMarginBottom = margin.bottom,
    pageMarginLeft = margin.left,
    pageMarginRight = margin.right,
  )
}

internal fun applyPageSizePreset(
  layout: LayoutMode.Paginated,
  preset: String,
  options: List<EditorPageMarginOption>,
): LayoutMode.Paginated {
  val option = EditorValues.pageLayout.firstOrNull { it.value == preset } ?: return layout
  val margin = pageMarginPresetOf(layout, options)
  val resized =
    layout.copy(pageWidth = option.layout.pageWidth, pageHeight = option.layout.pageHeight)

  return applyMarginPreset(resized, margin, option.margins)
}
