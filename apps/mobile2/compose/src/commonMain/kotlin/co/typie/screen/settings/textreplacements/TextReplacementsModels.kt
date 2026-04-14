package co.typie.screen.settings.textreplacements

// cspell:ignore SQUOTEOPEN SQUOTECLOSE DQUOTEOPEN DQUOTECLOSE

import co.typie.graphql.TextReplacementsScreen_Query
import co.typie.graphql.type.TextReplacementState

data class NormalizedTextReplacement(
  val textReplacementId: String,
  val preferenceId: String?,
  val match: String,
  val substitute: String,
  val regex: Boolean,
  val preset: Boolean,
  val state: TextReplacementState,
  val order: String?,
  val note: String?,
)

data class ReorderOrders(val lowerOrder: String?, val upperOrder: String?)

sealed interface TextReplacementFormError {
  val message: String

  data object EmptyMatch : TextReplacementFormError {
    override val message: String = "찾을 텍스트를 입력해주세요."
  }

  data object EmptySubstitute : TextReplacementFormError {
    override val message: String = "삽입할 텍스트를 입력해주세요."
  }

  data object IdenticalMatchAndSubstitute : TextReplacementFormError {
    override val message: String = "찾을 텍스트와 삽입할 텍스트가 같아요."
  }

  data object InvalidRegex : TextReplacementFormError {
    override val message: String = "정규식이 올바르지 않아요."
  }
}

const val SMART_QUOTE_OPEN_SINGLE_ID = "TXR0SQUOTEOPEN"
const val SMART_QUOTE_CLOSE_SINGLE_ID = "TXR0SQUOTECLOSE"
const val SMART_QUOTE_OPEN_DOUBLE_ID = "TXR0DQUOTEOPEN"
const val SMART_QUOTE_CLOSE_DOUBLE_ID = "TXR0DQUOTECLOSE"

private val SMART_QUOTE_ID_ORDER =
  listOf(
    SMART_QUOTE_OPEN_SINGLE_ID,
    SMART_QUOTE_CLOSE_SINGLE_ID,
    SMART_QUOTE_OPEN_DOUBLE_ID,
    SMART_QUOTE_CLOSE_DOUBLE_ID,
  )

private val SMART_QUOTE_ID_SET = SMART_QUOTE_ID_ORDER.toSet()

fun normalizeTextReplacement(
  textReplacement: TextReplacementsScreen_Query.TextReplacement
): NormalizedTextReplacement? {
  val direct = textReplacement.onTextReplacement
  if (direct != null) {
    return direct.toNormalizedTextReplacement()
  }

  val preference = textReplacement.onTextReplacementPreference
  if (preference != null) {
    return preference.toNormalizedTextReplacement()
  }

  return null
}

fun normalizeTextReplacements(
  textReplacements: List<TextReplacementsScreen_Query.TextReplacement>
): List<NormalizedTextReplacement> {
  return textReplacements.mapNotNull(::normalizeTextReplacement)
}

fun TextReplacementsScreen_Query.OnTextReplacement.toNormalizedTextReplacement():
  NormalizedTextReplacement {
  return NormalizedTextReplacement(
    textReplacementId = id,
    preferenceId = null,
    match = match,
    substitute = substitute,
    regex = regex,
    preset = preset,
    state = TextReplacementState.ACTIVE,
    order = order,
    note = note,
  )
}

fun TextReplacementsScreen_Query.OnTextReplacementPreference.toNormalizedTextReplacement():
  NormalizedTextReplacement {
  return NormalizedTextReplacement(
    textReplacementId = textReplacement.id,
    preferenceId = id,
    match = textReplacement.match,
    substitute = textReplacement.substitute,
    regex = textReplacement.regex,
    preset = textReplacement.preset,
    state = state,
    order = order ?: textReplacement.order,
    note = textReplacement.note,
  )
}

fun presetItems(items: List<NormalizedTextReplacement>): List<NormalizedTextReplacement> {
  return items.filter { item -> item.preset && !item.isSmartQuote }
}

fun smartQuoteItems(items: List<NormalizedTextReplacement>): List<NormalizedTextReplacement> {
  val byId = items.associateBy { it.textReplacementId }
  return SMART_QUOTE_ID_ORDER.mapNotNull { byId[it] }
}

fun customItems(items: List<NormalizedTextReplacement>): List<NormalizedTextReplacement> {
  return items.filterNot { it.preset }
}

fun isSmartQuoteEnabled(items: List<NormalizedTextReplacement>): Boolean {
  val smartQuoteItems = smartQuoteItems(items)
  return smartQuoteItems.size == SMART_QUOTE_ID_ORDER.size &&
    smartQuoteItems.all { it.state == TextReplacementState.ACTIVE }
}

fun validateTextReplacementForm(
  match: String,
  substitute: String,
  regex: Boolean,
  regexValidator: (String) -> Boolean,
): TextReplacementFormError? {
  val trimmedMatch = match.trim()
  if (trimmedMatch.isEmpty()) {
    return TextReplacementFormError.EmptyMatch
  }

  val trimmedSubstitute = substitute.trim()
  if (trimmedSubstitute.isEmpty()) {
    return TextReplacementFormError.EmptySubstitute
  }

  if (match == substitute) {
    return TextReplacementFormError.IdenticalMatchAndSubstitute
  }

  if (regex && !regexValidator(match)) {
    return TextReplacementFormError.InvalidRegex
  }

  return null
}

fun normalizedCustomItemIds(items: List<NormalizedTextReplacement>): List<String> {
  return customItems(items).map { it.textReplacementId }
}

fun calculateCustomReorderOrdersFromOrderedKeys(
  items: List<NormalizedTextReplacement>,
  orderedKeys: List<String>,
  movedKey: String,
): ReorderOrders? {
  val customItemsById = customItems(items).associateBy { it.textReplacementId }
  val orderedCustomItems = orderedKeys.mapNotNull(customItemsById::get)
  if (orderedCustomItems.size != customItemsById.size) {
    return null
  }

  val movedIndex = orderedCustomItems.indexOfFirst { item -> item.textReplacementId == movedKey }
  if (movedIndex == -1) {
    return null
  }

  return ReorderOrders(
    lowerOrder = orderedCustomItems.getOrNull(movedIndex - 1)?.order,
    upperOrder = orderedCustomItems.getOrNull(movedIndex + 1)?.order,
  )
}

private val NormalizedTextReplacement.isSmartQuote: Boolean
  get() = textReplacementId in SMART_QUOTE_ID_SET
