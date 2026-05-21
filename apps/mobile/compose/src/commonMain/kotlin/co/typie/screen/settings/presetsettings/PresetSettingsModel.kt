package co.typie.screen.settings.presetsettings

import co.typie.editor.ffi.LayoutMode
import kotlinx.serialization.EncodeDefault
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
internal sealed interface PresetPageLayout {
  @Serializable
  @SerialName("continuous")
  data class Continuous(@EncodeDefault val maxWidth: Int = 600) : PresetPageLayout

  @Serializable
  @SerialName("paginated")
  data class Paginated(
    @EncodeDefault val pageWidth: Int = 794,
    @EncodeDefault val pageHeight: Int = 1123,
    @EncodeDefault val pageMarginTop: Int = 94,
    @EncodeDefault val pageMarginBottom: Int = 94,
    @EncodeDefault val pageMarginLeft: Int = 94,
    @EncodeDefault val pageMarginRight: Int = 94,
  ) : PresetPageLayout
}

@Serializable
internal data class Preset(
  @EncodeDefault val fontFamily: String = "Pretendard",
  @EncodeDefault val fontSize: Int = 1200,
  @EncodeDefault val fontWeight: Int = 400,
  @EncodeDefault val textColor: String = "black",
  @EncodeDefault val backgroundColor: String = "none",
  @EncodeDefault val letterSpacing: Int = 0,
  @EncodeDefault val lineHeight: Int = 160,
  @EncodeDefault val layout: PresetPageLayout = PresetPageLayout.Paginated(),
  @EncodeDefault val paragraphIndent: Int = 100,
  @EncodeDefault val blockGap: Int = 100,
)

@Serializable internal data class PresetPreferences(@EncodeDefault val template: Preset? = Preset())

internal fun PresetPageLayout.toLayoutMode(): LayoutMode =
  when (this) {
    is PresetPageLayout.Continuous -> LayoutMode.Continuous(maxWidth = maxWidth)
    is PresetPageLayout.Paginated ->
      LayoutMode.Paginated(
        pageWidth = pageWidth,
        pageHeight = pageHeight,
        pageMarginTop = pageMarginTop,
        pageMarginBottom = pageMarginBottom,
        pageMarginLeft = pageMarginLeft,
        pageMarginRight = pageMarginRight,
      )
  }

internal fun LayoutMode.toPresetPageLayout(): PresetPageLayout =
  when (this) {
    is LayoutMode.Continuous -> PresetPageLayout.Continuous(maxWidth = maxWidth)
    is LayoutMode.Paginated ->
      PresetPageLayout.Paginated(
        pageWidth = pageWidth,
        pageHeight = pageHeight,
        pageMarginTop = pageMarginTop,
        pageMarginBottom = pageMarginBottom,
        pageMarginLeft = pageMarginLeft,
        pageMarginRight = pageMarginRight,
      )
  }
