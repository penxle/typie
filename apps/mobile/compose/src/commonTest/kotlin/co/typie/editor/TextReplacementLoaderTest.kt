package co.typie.editor

import co.typie.editor.ffi.RawTextReplacementRule
import co.typie.graphql.fragment.TextReplacementLoader_user
import co.typie.graphql.type.TextReplacementState
import kotlin.test.Test
import kotlin.test.assertEquals

class TextReplacementLoaderTest {
  private fun custom(id: String, match: String, substitute: String, regex: Boolean = false) =
    TextReplacementLoader_user.TextReplacement(
      __typename = "TextReplacement",
      onTextReplacement =
        TextReplacementLoader_user.OnTextReplacement(
          id = id,
          match = match,
          substitute = substitute,
          regex = regex,
        ),
      onTextReplacementPreference = null,
    )

  private fun preset(
    id: String,
    match: String,
    substitute: String,
    state: TextReplacementState,
    regex: Boolean = false,
  ) =
    TextReplacementLoader_user.TextReplacement(
      __typename = "TextReplacementPreference",
      onTextReplacement = null,
      onTextReplacementPreference =
        TextReplacementLoader_user.OnTextReplacementPreference(
          id = "$id-preference",
          state = state,
          textReplacement =
            TextReplacementLoader_user.TextReplacement1(
              __typename = "TextReplacement",
              id = id,
              match = match,
              substitute = substitute,
              regex = regex,
            ),
        ),
    )

  private fun user(vararg items: TextReplacementLoader_user.TextReplacement) =
    TextReplacementLoader_user(__typename = "User", id = "U1", textReplacements = items.toList())

  @Test
  fun `custom replacements map to rules in server order`() {
    val rules =
      user(custom("R1", "...", "⋯"), custom("R2", "a-z", "X", regex = true))
        .toTextReplacementRules()

    assertEquals(
      listOf(
        RawTextReplacementRule(id = "R1", matchPattern = "...", substitute = "⋯", regex = false),
        RawTextReplacementRule(id = "R2", matchPattern = "a-z", substitute = "X", regex = true),
      ),
      rules,
    )
  }

  @Test
  fun `active preference unwraps inner replacement`() {
    val rules = user(preset("R1", "...", "⋯", TextReplacementState.ACTIVE)).toTextReplacementRules()

    assertEquals(
      listOf(
        RawTextReplacementRule(id = "R1", matchPattern = "...", substitute = "⋯", regex = false)
      ),
      rules,
    )
  }

  @Test
  fun `disabled preference is excluded`() {
    val rules =
      user(
          preset("R1", "...", "⋯", TextReplacementState.DISABLED),
          custom("R2", "abc", "X"),
        )
        .toTextReplacementRules()

    assertEquals(
      listOf(
        RawTextReplacementRule(id = "R2", matchPattern = "abc", substitute = "X", regex = false)
      ),
      rules,
    )
  }
}
