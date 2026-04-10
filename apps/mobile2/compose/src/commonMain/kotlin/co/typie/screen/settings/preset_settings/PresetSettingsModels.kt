package co.typie.screen.settings.preset_settings

import co.typie.graphql.FontSettingsScreen_Query
import co.typie.graphql.PresetSettingsScreen_Query
import co.typie.graphql.type.FontFamilySource
import co.typie.graphql.type.FontFamilyState
import co.typie.graphql.type.FontState
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonPrimitive

internal enum class PresetSettingField {
  FontSize,
  PageSize,
  FontWeight,
  LetterSpacing,
  LineHeight,
  ParagraphIndent,
  BlockGap,
}

internal enum class PageMarginSide {
  Top,
  Bottom,
  Left,
  Right,
}

internal data class PresetFontEntry(
  val id: String,
  val weight: Int,
  val subfamilyDisplayName: String? = null,
  val state: FontState = FontState.ACTIVE,
)

internal data class PresetFontFamily(
  val id: String,
  val familyName: String,
  val displayName: String,
  val source: FontFamilySource,
  val state: FontFamilyState,
  val fonts: List<PresetFontEntry>,
)

internal data class PresetTemplate(
  val fontFamily: String = DEFAULT_FONT_FAMILY,
  val fontSize: Int = DEFAULT_FONT_SIZE,
  val fontWeight: Int = DEFAULT_FONT_WEIGHT,
  val textColor: String = DEFAULT_TEXT_COLOR,
  val backgroundColor: String = DEFAULT_BACKGROUND_COLOR,
  val letterSpacing: Int = DEFAULT_LETTER_SPACING,
  val lineHeight: Int = DEFAULT_LINE_HEIGHT,
  val layout: PresetLayout = PresetLayout.Continuous(),
  val paragraphIndent: Int = DEFAULT_PARAGRAPH_INDENT,
  val blockGap: Int = DEFAULT_BLOCK_GAP,
  val extra: Map<String, JsonElement> = emptyMap(),
  val presentKeys: Set<String> = emptySet(),
) {
  companion object {
    fun fromPreferencesJson(value: JsonElement): PresetTemplate {
      val preferences = value as? JsonObject ?: return PresetTemplate()
      val template = preferences["template"] as? JsonObject ?: return PresetTemplate()
      return fromJson(template)
    }

    fun fromJson(value: JsonElement): PresetTemplate {
      val objectValue = value as? JsonObject ?: return PresetTemplate()

      val knownKeys = setOf(
        "fontFamily",
        "fontSize",
        "fontWeight",
        "textColor",
        "backgroundColor",
        "letterSpacing",
        "lineHeight",
        "layout",
        "paragraphIndent",
        "blockGap",
      )

      return PresetTemplate(
        fontFamily = objectValue["fontFamily"].stringOrDefault(DEFAULT_FONT_FAMILY),
        fontSize = objectValue["fontSize"].intOrDefault(DEFAULT_FONT_SIZE),
        fontWeight = objectValue["fontWeight"].intOrDefault(DEFAULT_FONT_WEIGHT),
        textColor = objectValue["textColor"].stringOrDefault(DEFAULT_TEXT_COLOR),
        backgroundColor = objectValue["backgroundColor"].stringOrDefault(DEFAULT_BACKGROUND_COLOR),
        letterSpacing = objectValue["letterSpacing"].intOrDefault(DEFAULT_LETTER_SPACING),
        lineHeight = objectValue["lineHeight"].intOrDefault(DEFAULT_LINE_HEIGHT),
        layout = objectValue["layout"]?.let(PresetLayout::fromJson) ?: PresetLayout.Continuous(),
        paragraphIndent = objectValue["paragraphIndent"].intOrDefault(DEFAULT_PARAGRAPH_INDENT),
        blockGap = objectValue["blockGap"].intOrDefault(DEFAULT_BLOCK_GAP),
        extra = objectValue.filterKeys { key -> key !in knownKeys },
        presentKeys = objectValue.keys.intersect(knownKeys),
      )
    }
  }

  fun toJsonObject(): JsonObject {
    return buildJsonObject {
      if (presentKeys.contains("fontFamily") || fontFamily != DEFAULT_FONT_FAMILY) {
        put("fontFamily", JsonPrimitive(fontFamily))
      }
      if (presentKeys.contains("fontSize") || fontSize != DEFAULT_FONT_SIZE) {
        put("fontSize", JsonPrimitive(fontSize))
      }
      if (presentKeys.contains("fontWeight") || fontWeight != DEFAULT_FONT_WEIGHT) {
        put("fontWeight", JsonPrimitive(fontWeight))
      }
      if (presentKeys.contains("textColor") || textColor != DEFAULT_TEXT_COLOR) {
        put("textColor", JsonPrimitive(textColor))
      }
      if (presentKeys.contains("backgroundColor") || backgroundColor != DEFAULT_BACKGROUND_COLOR) {
        put("backgroundColor", JsonPrimitive(backgroundColor))
      }
      if (presentKeys.contains("letterSpacing") || letterSpacing != DEFAULT_LETTER_SPACING) {
        put("letterSpacing", JsonPrimitive(letterSpacing))
      }
      if (presentKeys.contains("lineHeight") || lineHeight != DEFAULT_LINE_HEIGHT) {
        put("lineHeight", JsonPrimitive(lineHeight))
      }
      if (presentKeys.contains("layout") || layout != PresetLayout.Continuous()) {
        put("layout", layout.toJsonElement())
      }
      if (presentKeys.contains("paragraphIndent") || paragraphIndent != DEFAULT_PARAGRAPH_INDENT) {
        put("paragraphIndent", JsonPrimitive(paragraphIndent))
      }
      if (presentKeys.contains("blockGap") || blockGap != DEFAULT_BLOCK_GAP) {
        put("blockGap", JsonPrimitive(blockGap))
      }
      extra.forEach { (key, value) -> put(key, value) }
    }
  }
}

internal fun PresetTemplate.withFontFamily(fontFamily: String): PresetTemplate {
  return copy(fontFamily = fontFamily, presentKeys = presentKeys + "fontFamily")
}

internal fun PresetTemplate.withFontSize(fontSize: Int): PresetTemplate {
  return copy(fontSize = fontSize, presentKeys = presentKeys + "fontSize")
}

internal fun PresetTemplate.withFontWeight(fontWeight: Int): PresetTemplate {
  return copy(fontWeight = fontWeight, presentKeys = presentKeys + "fontWeight")
}

internal fun PresetTemplate.withTextColor(textColor: String): PresetTemplate {
  return copy(textColor = textColor, presentKeys = presentKeys + "textColor")
}

internal fun PresetTemplate.withBackgroundColor(backgroundColor: String): PresetTemplate {
  return copy(backgroundColor = backgroundColor, presentKeys = presentKeys + "backgroundColor")
}

internal fun PresetTemplate.withLetterSpacing(letterSpacing: Int): PresetTemplate {
  return copy(letterSpacing = letterSpacing, presentKeys = presentKeys + "letterSpacing")
}

internal fun PresetTemplate.withLineHeight(lineHeight: Int): PresetTemplate {
  return copy(lineHeight = lineHeight, presentKeys = presentKeys + "lineHeight")
}

internal fun PresetTemplate.withLayout(layout: PresetLayout): PresetTemplate {
  return copy(layout = layout, presentKeys = presentKeys + "layout")
}

internal fun PresetTemplate.withParagraphIndent(paragraphIndent: Int): PresetTemplate {
  return copy(paragraphIndent = paragraphIndent, presentKeys = presentKeys + "paragraphIndent")
}

internal fun PresetTemplate.withBlockGap(blockGap: Int): PresetTemplate {
  return copy(blockGap = blockGap, presentKeys = presentKeys + "blockGap")
}

internal sealed interface PresetLayout {
  fun toJsonElement(): JsonElement

  data class Continuous(
    val maxWidth: Int = DEFAULT_MAX_WIDTH,
    val extra: Map<String, JsonElement> = emptyMap(),
    val presentKeys: Set<String> = emptySet(),
  ) : PresetLayout {
    override fun toJsonElement(): JsonElement {
      return buildJsonObject {
        put("type", JsonPrimitive("continuous"))
        if (presentKeys.contains("maxWidth") || maxWidth != DEFAULT_MAX_WIDTH) {
          put("maxWidth", JsonPrimitive(maxWidth))
        }
        extra.forEach { (key, value) -> put(key, value) }
      }
    }
  }

  data class Paginated(
    val pageWidth: Int,
    val pageHeight: Int,
    val pageMarginTop: Int,
    val pageMarginBottom: Int,
    val pageMarginLeft: Int,
    val pageMarginRight: Int,
    val extra: Map<String, JsonElement> = emptyMap(),
    val presentKeys: Set<String> = emptySet(),
  ) : PresetLayout {
    override fun toJsonElement(): JsonElement {
      return buildJsonObject {
        put("type", JsonPrimitive("paginated"))
        if (presentKeys.contains("pageWidth") || pageWidth != createPaginatedLayout("a4").pageWidth) {
          put("pageWidth", JsonPrimitive(pageWidth))
        }
        if (presentKeys.contains("pageHeight") || pageHeight != createPaginatedLayout("a4").pageHeight) {
          put("pageHeight", JsonPrimitive(pageHeight))
        }
        if (presentKeys.contains("pageMarginTop") || pageMarginTop != createPaginatedLayout("a4").pageMarginTop) {
          put("pageMarginTop", JsonPrimitive(pageMarginTop))
        }
        if (presentKeys.contains("pageMarginBottom") || pageMarginBottom != createPaginatedLayout("a4").pageMarginBottom) {
          put("pageMarginBottom", JsonPrimitive(pageMarginBottom))
        }
        if (presentKeys.contains("pageMarginLeft") || pageMarginLeft != createPaginatedLayout("a4").pageMarginLeft) {
          put("pageMarginLeft", JsonPrimitive(pageMarginLeft))
        }
        if (presentKeys.contains("pageMarginRight") || pageMarginRight != createPaginatedLayout("a4").pageMarginRight) {
          put("pageMarginRight", JsonPrimitive(pageMarginRight))
        }
        extra.forEach { (key, value) -> put(key, value) }
      }
    }
  }

  data class Unknown(
    val raw: JsonElement,
  ) : PresetLayout {
    override fun toJsonElement(): JsonElement = raw
  }

  companion object {
    fun fromJson(value: JsonElement): PresetLayout {
      val objectValue = value as? JsonObject ?: return PresetLayout.Unknown(value)
      val type = objectValue["type"].stringOrNull() ?: return PresetLayout.Unknown(objectValue)

      return when (type) {
        "continuous" -> {
          val knownKeys = setOf("type", "maxWidth")
          PresetLayout.Continuous(
            maxWidth = objectValue["maxWidth"].intOrDefault(DEFAULT_MAX_WIDTH),
            extra = objectValue.filterKeys { key -> key !in knownKeys },
            presentKeys = objectValue.keys.intersect(knownKeys),
          )
        }
        "paginated" -> {
          val knownKeys = setOf(
            "type",
            "pageWidth",
            "pageHeight",
            "pageMarginTop",
            "pageMarginBottom",
            "pageMarginLeft",
            "pageMarginRight",
          )
          val defaultLayout = createPaginatedLayout("a4")
          PresetLayout.Paginated(
            pageWidth = objectValue["pageWidth"].intOrDefault(defaultLayout.pageWidth),
            pageHeight = objectValue["pageHeight"].intOrDefault(defaultLayout.pageHeight),
            pageMarginTop = objectValue["pageMarginTop"].intOrDefault(defaultLayout.pageMarginTop),
            pageMarginBottom = objectValue["pageMarginBottom"].intOrDefault(defaultLayout.pageMarginBottom),
            pageMarginLeft = objectValue["pageMarginLeft"].intOrDefault(defaultLayout.pageMarginLeft),
            pageMarginRight = objectValue["pageMarginRight"].intOrDefault(defaultLayout.pageMarginRight),
            extra = objectValue.filterKeys { key -> key !in knownKeys },
            presentKeys = objectValue.keys.intersect(knownKeys),
          )
        }
        else -> PresetLayout.Unknown(objectValue)
      }
    }
  }
}

internal fun PresetLayout.Continuous.withMaxWidth(maxWidth: Int): PresetLayout.Continuous {
  return copy(maxWidth = maxWidth, presentKeys = presentKeys + "maxWidth")
}

internal fun PresetLayout.Paginated.withPageWidth(pageWidth: Int): PresetLayout.Paginated {
  return copy(pageWidth = pageWidth, presentKeys = presentKeys + "pageWidth")
}

internal fun PresetLayout.Paginated.withPageHeight(pageHeight: Int): PresetLayout.Paginated {
  return copy(pageHeight = pageHeight, presentKeys = presentKeys + "pageHeight")
}

internal fun PresetLayout.Paginated.withPageMarginTop(pageMarginTop: Int): PresetLayout.Paginated {
  return copy(pageMarginTop = pageMarginTop, presentKeys = presentKeys + "pageMarginTop")
}

internal fun PresetLayout.Paginated.withPageMarginBottom(pageMarginBottom: Int): PresetLayout.Paginated {
  return copy(pageMarginBottom = pageMarginBottom, presentKeys = presentKeys + "pageMarginBottom")
}

internal fun PresetLayout.Paginated.withPageMarginLeft(pageMarginLeft: Int): PresetLayout.Paginated {
  return copy(pageMarginLeft = pageMarginLeft, presentKeys = presentKeys + "pageMarginLeft")
}

internal fun PresetLayout.Paginated.withPageMarginRight(pageMarginRight: Int): PresetLayout.Paginated {
  return copy(pageMarginRight = pageMarginRight, presentKeys = presentKeys + "pageMarginRight")
}

internal fun activePresetFontFamiliesFromFontSettingsQuery(
  families: List<FontSettingsScreen_Query.DocumentFontFamily>,
): List<PresetFontFamily> {
  return families
    .filter { it.state == FontFamilyState.ACTIVE }
    .map { family ->
      family.toPresetFontFamily().copy(
        fonts = family.fonts
          .filter { it.state == FontState.ACTIVE }
          .sortedBy { it.weight }
          .associateBy { it.weight }
          .values
          .map { it.toPresetFontEntry() },
      )
    }
    .filter { it.fonts.isNotEmpty() }
}

internal fun activePresetFontFamiliesFromPresetQuery(
  families: List<PresetSettingsScreen_Query.DocumentFontFamily>,
): List<PresetFontFamily> {
  return families
    .filter { it.state == FontFamilyState.ACTIVE }
    .map { family ->
      family.toPresetFontFamily().copy(
        fonts = family.fonts
          .filter { it.state == FontState.ACTIVE }
          .sortedBy { it.weight }
          .associateBy { it.weight }
          .values
          .map { it.toPresetFontEntry() },
      )
    }
    .filter { it.fonts.isNotEmpty() }
}

internal fun normalizedPresetFontFamilyOptions(
  families: List<PresetFontFamily>,
): List<PresetOption<String>> {
  return families
    .sortedWith(compareBy<PresetFontFamily> { it.displayName.lowercase() }.thenBy { it.familyName.lowercase() })
    .map { family ->
      PresetOption(
        label = family.displayName,
        value = family.familyName,
      )
    }
}

internal fun selectedFontWeightAvailabilityOptions(
  template: PresetTemplate,
  families: List<PresetFontFamily>,
): List<PresetOption<Int>> {
  val selectedFamily = families.firstOrNull { it.familyName == template.fontFamily } ?: return emptyList()

  return selectedFamily.fonts
    .distinctBy { it.weight }
    .sortedBy { it.weight }
    .map { font ->
      PresetOption(
        label = fontWeightLabel(font.weight, font.subfamilyDisplayName),
        value = font.weight,
      )
    }
}

internal fun FontSettingsScreen_Query.DocumentFontFamily.toPresetFontFamily(): PresetFontFamily {
  return PresetFontFamily(
    id = id,
    familyName = familyName,
    displayName = displayName,
    source = source,
    state = state,
    fonts = fonts.map { it.toPresetFontEntry() },
  )
}

internal fun PresetSettingsScreen_Query.DocumentFontFamily.toPresetFontFamily(): PresetFontFamily {
  return PresetFontFamily(
    id = id,
    familyName = familyName,
    displayName = displayName,
    source = source,
    state = state,
    fonts = fonts.map { it.toPresetFontEntry() },
  )
}

internal fun FontSettingsScreen_Query.Font.toPresetFontEntry(): PresetFontEntry {
  return PresetFontEntry(
    id = id,
    weight = weight,
    subfamilyDisplayName = subfamilyDisplayName,
    state = state,
  )
}

internal fun PresetSettingsScreen_Query.Font.toPresetFontEntry(): PresetFontEntry {
  return PresetFontEntry(
    id = id,
    weight = weight,
    subfamilyDisplayName = subfamilyDisplayName,
    state = state,
  )
}

private fun fontWeightLabel(
  weight: Int,
  subfamilyDisplayName: String?,
): String {
  return FONT_WEIGHT_OPTIONS.firstOrNull { it.value == weight }?.label
    ?: subfamilyDisplayName?.takeIf { it.isNotBlank() }?.let { "$it ($weight)" }
    ?: weight.toString()
}

internal fun closestWeight(
  targetWeight: Int,
  weights: Iterable<Int>,
): Int {
  return weights
    .distinct()
    .minWithOrNull(
      compareBy<Int> { kotlin.math.abs(targetWeight - it) }
        .thenByDescending { it },
    )
    ?: targetWeight
}

internal fun <T> presetOptionLabel(
  options: List<PresetOption<T>>,
  value: T,
  fallback: String = value.toString(),
): String {
  return options.firstOrNull { it.value == value }?.label ?: fallback
}

internal fun layoutModeSummaryLabel(layout: PresetLayout): String {
  return when (layout) {
    is PresetLayout.Continuous -> "스크롤"
    is PresetLayout.Paginated -> "페이지"
    is PresetLayout.Unknown -> "사용자 지정"
  }
}

internal fun fontSizeSummaryLabel(fontSize: Int): String {
  return "${formatPresetPointValue(fontSize)}pt"
}

internal fun pageLayoutPresetOrCustom(layout: PresetLayout.Paginated): String {
  return PAGE_LAYOUT_OPTIONS.firstOrNull { it.layout.matchesPageSize(layout) }?.value ?: "custom"
}

internal fun pageLayoutSummaryLabel(layout: PresetLayout.Paginated): String {
  return PAGE_LAYOUT_OPTIONS
    .firstOrNull { it.layout.matchesPageSize(layout) }
    ?.label
    ?.substringBefore(" (")
    ?: "${pxToMm(layout.pageWidth)} × ${pxToMm(layout.pageHeight)}mm"
}

internal fun pageMarginSummaryLabel(layout: PresetLayout.Paginated): String {
  return "${pxToMm(layout.pageMarginTop)}·${pxToMm(layout.pageMarginBottom)}·${pxToMm(layout.pageMarginLeft)}·${pxToMm(layout.pageMarginRight)}mm"
}

internal fun presetSettingSupportsSecondaryInput(field: PresetSettingField): Boolean {
  return field == PresetSettingField.FontSize || field == PresetSettingField.PageSize
}

private fun PresetLayout.Paginated.matchesPageSize(other: PresetLayout.Paginated): Boolean {
  return pageWidth == other.pageWidth &&
    pageHeight == other.pageHeight
}

private fun PresetLayout.Paginated.matches(other: PresetLayout.Paginated): Boolean {
  return pageWidth == other.pageWidth &&
    pageHeight == other.pageHeight &&
    pageMarginTop == other.pageMarginTop &&
    pageMarginBottom == other.pageMarginBottom &&
    pageMarginLeft == other.pageMarginLeft &&
    pageMarginRight == other.pageMarginRight
}

private fun JsonElement?.stringOrDefault(default: String): String {
  return this?.jsonPrimitive?.contentOrNull ?: default
}

private fun JsonElement?.intOrDefault(default: Int): Int {
  return this?.jsonPrimitive?.intOrNull ?: default
}

private fun JsonElement?.stringOrNull(): String? = this?.jsonPrimitive?.contentOrNull
