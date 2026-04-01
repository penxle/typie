package co.typie.screen.text_replacements

import co.typie.graphql.TextReplacementsScreen_Query
import co.typie.graphql.type.TextReplacementState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class TextReplacementsModelsTest {
  @Test
  fun `normalizeTextReplacement maps plain union item`() {
    val normalized = normalizeTextReplacement(
      textReplacement = TextReplacementsScreen_Query.TextReplacement(
        __typename = "TextReplacement",
        onTextReplacement = TextReplacementsScreen_Query.OnTextReplacement(
          id = "plain-id",
          match = "a",
          substitute = "b",
          regex = false,
          preset = true,
          note = "plain note",
          order = "10",
        ),
        onTextReplacementPreference = null,
      ),
    )

    assertEquals(
      NormalizedTextReplacement(
        textReplacementId = "plain-id",
        preferenceId = null,
        match = "a",
        substitute = "b",
        regex = false,
        preset = true,
        state = TextReplacementState.ACTIVE,
        order = "10",
        note = "plain note",
      ),
      normalized,
    )
  }

  @Test
  fun `normalizeTextReplacement maps preference union item`() {
    val normalized = normalizeTextReplacement(
      textReplacement = TextReplacementsScreen_Query.TextReplacement(
        __typename = "TextReplacementPreference",
        onTextReplacement = null,
        onTextReplacementPreference = TextReplacementsScreen_Query.OnTextReplacementPreference(
          id = "pref-id",
          state = TextReplacementState.DISABLED,
          order = "20",
          textReplacement = TextReplacementsScreen_Query.TextReplacement1(
            __typename = "TextReplacement",
            id = "replacement-id",
            match = "c",
            substitute = "d",
            regex = true,
            preset = false,
            note = null,
            order = "15",
          ),
        ),
      ),
    )

    assertEquals(
      NormalizedTextReplacement(
        textReplacementId = "replacement-id",
        preferenceId = "pref-id",
        match = "c",
        substitute = "d",
        regex = true,
        preset = false,
        state = TextReplacementState.DISABLED,
        order = "20",
        note = null,
      ),
      normalized,
    )
  }

  @Test
  fun `preset custom and smart quote partition helpers split normalized items`() {
    val items = listOf(
      normalizedItem(
        id = "preset-id",
        match = "match",
        substitute = "substitute",
        preset = true,
        state = TextReplacementState.ACTIVE,
        order = "10",
      ),
      normalizedItem(
        id = SMART_QUOTE_OPEN_SINGLE_ID,
        match = "'",
        substitute = "‘",
        preset = true,
        state = TextReplacementState.ACTIVE,
        order = "11",
      ),
      normalizedItem(
        id = "custom-id",
        match = "foo",
        substitute = "bar",
        preset = false,
        state = TextReplacementState.DISABLED,
        order = "12",
      ),
    )

    assertEquals(listOf("preset-id"), presetItems(items).map { it.textReplacementId })
    assertEquals(listOf(SMART_QUOTE_OPEN_SINGLE_ID), smartQuoteItems(items).map { it.textReplacementId })
    assertEquals(listOf("custom-id"), customItems(items).map { it.textReplacementId })
  }

  @Test
  fun `smart quote items group the four preset ids`() {
    val items = listOf(
      normalizedItem(id = "other-preset", preset = true, state = TextReplacementState.ACTIVE),
      normalizedItem(id = SMART_QUOTE_OPEN_SINGLE_ID, preset = true, state = TextReplacementState.ACTIVE),
      normalizedItem(id = SMART_QUOTE_CLOSE_SINGLE_ID, preset = true, state = TextReplacementState.ACTIVE),
      normalizedItem(id = SMART_QUOTE_OPEN_DOUBLE_ID, preset = true, state = TextReplacementState.ACTIVE),
      normalizedItem(id = SMART_QUOTE_CLOSE_DOUBLE_ID, preset = true, state = TextReplacementState.ACTIVE),
    )

    assertEquals(
      listOf(
        SMART_QUOTE_OPEN_SINGLE_ID,
        SMART_QUOTE_CLOSE_SINGLE_ID,
        SMART_QUOTE_OPEN_DOUBLE_ID,
        SMART_QUOTE_CLOSE_DOUBLE_ID,
      ),
      smartQuoteItems(items).map { it.textReplacementId },
    )
  }

  @Test
  fun `isSmartQuoteEnabled returns true only when all four smart quote rules are active`() {
    val activeItems = smartQuoteItems(
      listOf(
        normalizedItem(id = SMART_QUOTE_OPEN_SINGLE_ID, preset = true, state = TextReplacementState.ACTIVE),
        normalizedItem(id = SMART_QUOTE_CLOSE_SINGLE_ID, preset = true, state = TextReplacementState.ACTIVE),
        normalizedItem(id = SMART_QUOTE_OPEN_DOUBLE_ID, preset = true, state = TextReplacementState.ACTIVE),
        normalizedItem(id = SMART_QUOTE_CLOSE_DOUBLE_ID, preset = true, state = TextReplacementState.ACTIVE),
      ),
    )
    val disabledItems = activeItems.toMutableList().also {
      it[2] = it[2].copy(state = TextReplacementState.DISABLED)
    }

    assertTrue(isSmartQuoteEnabled(activeItems))
    assertFalse(isSmartQuoteEnabled(disabledItems))
  }

  @Test
  fun `validateTextReplacementForm rejects blank and duplicate values`() {
    val regexValidator: (String) -> Boolean = { true }

    assertEquals(
      TextReplacementFormError.EmptyMatch,
      validateTextReplacementForm(
        match = "   ",
        substitute = "ok",
        regex = false,
        regexValidator = regexValidator,
      ),
    )

    assertEquals(
      TextReplacementFormError.EmptySubstitute,
      validateTextReplacementForm(
        match = "ok",
        substitute = "   ",
        regex = false,
        regexValidator = regexValidator,
      ),
    )

    assertEquals(
      TextReplacementFormError.IdenticalMatchAndSubstitute,
      validateTextReplacementForm(
        match = "same",
        substitute = "same",
        regex = false,
        regexValidator = regexValidator,
      ),
    )
  }

  @Test
  fun `validateTextReplacementForm allows whitespace significant values`() {
    val result = validateTextReplacementForm(
      match = " a",
      substitute = "a ",
      regex = false,
      regexValidator = { true },
    )

    assertEquals(null, result)
  }

  @Test
  fun `validateTextReplacementForm uses injected regex validator`() {
    val errors = mutableListOf<String>()
    val validator: (String) -> Boolean = { pattern ->
      errors += pattern
      false
    }

    assertEquals(
      TextReplacementFormError.InvalidRegex,
      validateTextReplacementForm(
        match = "[",
        substitute = "ok",
        regex = true,
        regexValidator = validator,
      ),
    )
    assertEquals(listOf("["), errors)
  }

  @Test
  fun `calculateCustomReorderOrdersFromOrderedKeys returns lower and upper neighbors for moved custom items`() {
    val items = listOf(
      normalizedItem(id = "custom-1", preset = false, order = "10"),
      normalizedItem(id = "custom-2", preset = false, order = "20"),
      normalizedItem(id = "custom-3", preset = false, order = "30"),
    )

    assertEquals(
      ReorderOrders(lowerOrder = "20", upperOrder = "30"),
      calculateCustomReorderOrdersFromOrderedKeys(
        items = items,
        orderedKeys = listOf("custom-2", "custom-1", "custom-3"),
        movedKey = "custom-1",
      ),
    )
    assertEquals(
      ReorderOrders(lowerOrder = null, upperOrder = "10"),
      calculateCustomReorderOrdersFromOrderedKeys(
        items = items,
        orderedKeys = listOf("custom-2", "custom-1", "custom-3"),
        movedKey = "custom-2",
      ),
    )
    assertEquals(
      ReorderOrders(lowerOrder = "30", upperOrder = null),
      calculateCustomReorderOrdersFromOrderedKeys(
        items = items,
        orderedKeys = listOf("custom-1", "custom-3", "custom-2"),
        movedKey = "custom-2",
      ),
    )
  }

  @Test
  fun `normalizeTextReplacement falls back to nested preference order when outer order is null`() {
    val normalized = normalizeTextReplacement(
      textReplacement = TextReplacementsScreen_Query.TextReplacement(
        __typename = "TextReplacementPreference",
        onTextReplacement = null,
        onTextReplacementPreference = TextReplacementsScreen_Query.OnTextReplacementPreference(
          id = "pref-id",
          state = TextReplacementState.ACTIVE,
          order = null,
          textReplacement = TextReplacementsScreen_Query.TextReplacement1(
            __typename = "TextReplacement",
            id = "replacement-id",
            match = "c",
            substitute = "d",
            regex = false,
            preset = true,
            note = null,
            order = "nested-order",
          ),
        ),
      ),
    )

    assertEquals("nested-order", normalized?.order)
  }

  private fun normalizedItem(
    id: String,
    match: String = "match",
    substitute: String = "substitute",
    preset: Boolean,
    state: TextReplacementState = TextReplacementState.ACTIVE,
    order: String? = null,
    preferenceId: String? = null,
  ): NormalizedTextReplacement {
    return NormalizedTextReplacement(
      textReplacementId = id,
      preferenceId = preferenceId,
      match = match,
      substitute = substitute,
      regex = false,
      preset = preset,
      state = state,
      order = order,
      note = null,
    )
  }
}
