package co.typie.screen.text_replacements

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.form.FormState
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TextReplacementsScreen_CreateTextReplacement_Mutation
import co.typie.graphql.TextReplacementsScreen_DeleteTextReplacement_Mutation
import co.typie.graphql.TextReplacementsScreen_MoveTextReplacement_Mutation
import co.typie.graphql.TextReplacementsScreen_Query
import co.typie.graphql.TextReplacementsScreen_UpdateTextReplacement_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.CreateTextReplacementInput
import co.typie.graphql.type.DeleteTextReplacementInput
import co.typie.graphql.type.MoveTextReplacementInput
import co.typie.graphql.type.TextReplacementState
import co.typie.graphql.type.UpdateTextReplacementInput
import co.typie.graphql.type.buildUser
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.CoroutineScope
import org.koin.core.annotation.KoinViewModel

class TextReplacementForm(
  scope: CoroutineScope,
  editingItem: NormalizedTextReplacement?,
) : FormState(scope) {
  val match = field(editingItem?.match.orEmpty())
  val substitute = field(editingItem?.substitute.orEmpty())
  val note = field(editingItem?.note.orEmpty())
  val regex = field(editingItem?.regex ?: false) {
    focusable = false
  }
}

sealed interface SaveRuleError {
  data class ValidationFailed(val message: String) : SaveRuleError
}

@KoinViewModel
class TextReplacementsViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val query = apolloClient.watchQuery(scope = viewModelScope, placeholderData()) { TextReplacementsScreen_Query() }

  val normalizedItems: List<NormalizedTextReplacement>
    get() = normalizeTextReplacements(query.data.me.textReplacements)

  val normalizedPresetItems: List<NormalizedTextReplacement>
    get() = presetItems(normalizedItems)

  val normalizedSmartQuoteItems: List<NormalizedTextReplacement>
    get() = smartQuoteItems(normalizedItems)

  val normalizedCustomItems: List<NormalizedTextReplacement>
    get() = customItems(normalizedItems)

  val isSmartQuoteEnabled: Boolean
    get() = isSmartQuoteEnabled(normalizedItems)

  fun validateRegex(pattern: String): Boolean {
    return true
//    return editorEngine.validateRegex(pattern)
  }

  suspend fun togglePreset(item: NormalizedTextReplacement): Result<Unit, Nothing> {
    return toggleTextReplacementState(item = item, enabled = item.state != TextReplacementState.ACTIVE)
  }

  suspend fun toggleSmartQuotes(
    items: List<NormalizedTextReplacement>,
    enabled: Boolean,
  ): Result<Unit, Nothing> = result {
    val smartQuoteItems = smartQuoteItems(items)
    if (smartQuoteItems.all { it.state == desiredState(enabled) }) return@result

    smartQuoteItems.forEach { item ->
      updateTextReplacementState(
        textReplacementId = item.textReplacementId,
        enabled = enabled,
      )
    }
    query.refetch()
  }

  suspend fun saveCustomRule(
    editingItem: NormalizedTextReplacement?,
    match: String,
    substitute: String,
    regex: Boolean,
    note: String?,
    lastOrder: String?,
  ): Result<Unit, SaveRuleError> = result {
    val validationError = validateTextReplacementForm(
      match = match,
      substitute = substitute,
      regex = regex,
      regexValidator = ::validateRegex,
    )
    if (validationError != null) {
      raise(SaveRuleError.ValidationFailed(validationError.message))
    }

    val normalizedNote = note?.takeIf { it.isNotBlank() }

    if (editingItem == null) {
      val builder = CreateTextReplacementInput.Builder()
        .match(match)
        .substitute(substitute)
        .regex(regex)
        .note(normalizedNote)

      if (lastOrder != null) {
        builder.lowerOrder(lastOrder)
      }

      apolloClient.executeMutation(
        TextReplacementsScreen_CreateTextReplacement_Mutation(
          input = builder.build(),
        ),
      )
    } else {
      apolloClient.executeMutation(
        TextReplacementsScreen_UpdateTextReplacement_Mutation(
          input = UpdateTextReplacementInput.Builder()
            .textReplacementId(editingItem.textReplacementId)
            .match(match)
            .substitute(substitute)
            .regex(regex)
            .note(normalizedNote)
            .state(editingItem.state)
            .build(),
        ),
      )
    }

    query.refetch()
  }

  suspend fun toggleCustom(item: NormalizedTextReplacement): Result<Unit, Nothing> {
    return toggleTextReplacementState(item = item, enabled = item.state != TextReplacementState.ACTIVE)
  }

  suspend fun deleteCustom(item: NormalizedTextReplacement): Result<Unit, Nothing> = result {
    apolloClient.executeMutation(
      TextReplacementsScreen_DeleteTextReplacement_Mutation(
        input = DeleteTextReplacementInput.Builder()
          .textReplacementId(item.textReplacementId)
          .build(),
      ),
    )
    query.refetch()
  }

  suspend fun moveCustom(
    textReplacementId: String,
    lowerOrder: String?,
    upperOrder: String?,
  ): Result<Unit, Nothing> = result {
    apolloClient.executeMutation(
      TextReplacementsScreen_MoveTextReplacement_Mutation(
        input = MoveTextReplacementInput.Builder()
          .textReplacementId(textReplacementId)
          .apply {
            if (lowerOrder != null) {
              lowerOrder(lowerOrder)
            }
            if (upperOrder != null) {
              upperOrder(upperOrder)
            }
          }
          .build(),
      ),
    )
    query.refetch()
  }

  private suspend fun toggleTextReplacementState(
    item: NormalizedTextReplacement,
    enabled: Boolean,
  ): Result<Unit, Nothing> = result {
    if (item.state == desiredState(enabled)) return@result

    updateTextReplacementState(
      textReplacementId = item.textReplacementId,
      enabled = enabled,
    )
    query.refetch()
  }

  private suspend fun updateTextReplacementState(
    textReplacementId: String,
    enabled: Boolean,
  ) {
    apolloClient.executeMutation(
      TextReplacementsScreen_UpdateTextReplacement_Mutation(
        input = UpdateTextReplacementInput.Builder()
          .textReplacementId(textReplacementId)
          .state(desiredState(enabled))
          .build(),
      ),
    )
  }

  private fun desiredState(enabled: Boolean): TextReplacementState {
    return if (enabled) TextReplacementState.ACTIVE else TextReplacementState.DISABLED
  }
}

private fun placeholderData() = TextReplacementsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    textReplacements = emptyList()
  }
}
