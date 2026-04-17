package co.typie.screen.settings.textreplacements

// cspell:ignore SQUOTEOPEN SQUOTECLOSE DQUOTEOPEN DQUOTECLOSE

import co.typie.graphql.TextReplacementsScreen_Query
import co.typie.graphql.type.TextReplacementState

internal typealias TextReplacement = TextReplacementsScreen_Query.TextReplacement

internal typealias CustomTextReplacement = TextReplacementsScreen_Query.OnTextReplacement

internal val SMART_QUOTE_IDS =
  listOf("TXR0SQUOTEOPEN", "TXR0SQUOTECLOSE", "TXR0DQUOTEOPEN", "TXR0DQUOTECLOSE")

val TextReplacement.textReplacementId: String
  get() = onTextReplacement?.id ?: onTextReplacementPreference!!.textReplacement.id

val TextReplacement.match: String
  get() = onTextReplacement?.match ?: onTextReplacementPreference!!.textReplacement.match

val TextReplacement.substitute: String
  get() = onTextReplacement?.substitute ?: onTextReplacementPreference!!.textReplacement.substitute

val TextReplacement.regex: Boolean
  get() = onTextReplacement?.regex ?: onTextReplacementPreference!!.textReplacement.regex

val TextReplacement.note: String?
  get() = onTextReplacement?.note ?: onTextReplacementPreference!!.textReplacement.note

val TextReplacement.order: String?
  get() =
    onTextReplacement?.order
      ?: onTextReplacementPreference?.let { it.order ?: it.textReplacement.order }

val TextReplacement.isPreset: Boolean
  get() = onTextReplacement?.preset == true || onTextReplacementPreference != null

val TextReplacement.isCustom: Boolean
  get() = onTextReplacement?.preset == false

val TextReplacement.isSmartQuote: Boolean
  get() = textReplacementId in SMART_QUOTE_IDS

val TextReplacement.isActive: Boolean
  get() = onTextReplacementPreference?.state != TextReplacementState.DISABLED

internal fun applyOptimisticOrder(
  serverItems: List<TextReplacement>,
  optimisticKeys: List<String>?,
): List<TextReplacement> {
  if (optimisticKeys == null) return serverItems
  val byId = serverItems.associateBy { it.textReplacementId }
  val ordered = optimisticKeys.mapNotNull(byId::get)
  if (ordered.size == serverItems.size) return ordered
  val orderedIds = ordered.mapTo(mutableSetOf()) { it.textReplacementId }
  return ordered + serverItems.filterNot { it.textReplacementId in orderedIds }
}

internal fun neighboringOrders(
  orderedKeys: List<String>,
  movedKey: String,
  items: List<TextReplacement>,
): Pair<String?, String?>? {
  val byId = items.associateBy { it.textReplacementId }
  val ordered = orderedKeys.mapNotNull(byId::get)
  if (ordered.size != items.size) return null
  val idx = ordered.indexOfFirst { it.textReplacementId == movedKey }
  if (idx < 0) return null
  return ordered.getOrNull(idx - 1)?.order to ordered.getOrNull(idx + 1)?.order
}

internal fun isValidRegex(pattern: String): Boolean = runCatching { Regex(pattern) }.isSuccess
