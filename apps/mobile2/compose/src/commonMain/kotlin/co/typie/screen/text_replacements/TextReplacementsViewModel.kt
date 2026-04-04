package co.typie.screen.text_replacements

import co.touchlab.kermit.Logger
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TextReplacementsScreen_CreateTextReplacement_Mutation
import co.typie.graphql.TextReplacementsScreen_DeleteTextReplacement_Mutation
import co.typie.graphql.TextReplacementsScreen_MoveTextReplacement_Mutation
import co.typie.graphql.TextReplacementsScreen_Query
import co.typie.graphql.TextReplacementsScreen_UpdateTextReplacement_Mutation
import co.typie.graphql.type.CreateTextReplacementInput
import co.typie.graphql.type.DeleteTextReplacementInput
import co.typie.graphql.type.MoveTextReplacementInput
import co.typie.graphql.type.TextReplacementState
import co.typie.graphql.type.UpdateTextReplacementInput
import co.typie.graphql.type.buildUser
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import kotlinx.coroutines.CancellationException
import org.koin.core.annotation.KoinViewModel

private const val MUTATION_FAILURE_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."

@KoinViewModel
class TextReplacementsViewModel(
  private val toast: Toast,
) : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { TextReplacementsScreen_Query() }

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

  suspend fun togglePreset(item: NormalizedTextReplacement) {
    toggleTextReplacementState(item = item, enabled = item.state != TextReplacementState.ACTIVE)
  }

  suspend fun toggleSmartQuotes(
    items: List<NormalizedTextReplacement>,
    enabled: Boolean,
  ) {
    val smartQuoteItems = smartQuoteItems(items)
    if (smartQuoteItems.all { it.state == desiredState(enabled) }) return

    try {
      smartQuoteItems.forEach { item ->
        updateTextReplacementState(
          textReplacementId = item.textReplacementId,
          enabled = enabled,
        )
      }
      query.refetch()
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to toggle smart quotes" }
      try {
        query.refetch()
      } catch (refetchError: CancellationException) {
        throw refetchError
      } catch (refetchError: Exception) {
        Logger.e(refetchError) { "Failed to refetch text replacements after partial smart quote toggle failure" }
      }
      toast.show(ToastType.Error, MUTATION_FAILURE_MESSAGE)
    }
  }

  suspend fun saveCustomRule(
    editingItem: NormalizedTextReplacement?,
    match: String,
    substitute: String,
    regex: Boolean,
    note: String?,
    lastOrder: String?,
  ): Boolean {
    val validationError = validateTextReplacementForm(
      match = match,
      substitute = substitute,
      regex = regex,
      regexValidator = ::validateRegex,
    )
    if (validationError != null) {
      toast.show(ToastType.Error, validationError.message)
      return false
    }

    val normalizedNote = note?.takeIf { it.isNotBlank() }

    try {
      if (editingItem == null) {
        val builder = CreateTextReplacementInput.Builder()
          .match(match)
          .substitute(substitute)
          .regex(regex)
          .note(normalizedNote)

        if (lastOrder != null) {
          builder.lowerOrder(lastOrder)
        }

        executeMutation(
          TextReplacementsScreen_CreateTextReplacement_Mutation(
            input = builder.build(),
          ),
        )
      } else {
        executeMutation(
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
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to save custom text replacement" }
      toast.show(ToastType.Error, MUTATION_FAILURE_MESSAGE)
      return false
    }
  }

  suspend fun toggleCustom(item: NormalizedTextReplacement) {
    toggleTextReplacementState(item = item, enabled = item.state != TextReplacementState.ACTIVE)
  }

  suspend fun deleteCustom(item: NormalizedTextReplacement): Boolean {
    try {
      executeMutation(
        TextReplacementsScreen_DeleteTextReplacement_Mutation(
          input = DeleteTextReplacementInput.Builder()
            .textReplacementId(item.textReplacementId)
            .build(),
        ),
      )
      query.refetch()
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to delete custom text replacement" }
      toast.show(ToastType.Error, MUTATION_FAILURE_MESSAGE)
      return false
    }
  }

  suspend fun moveCustom(
    textReplacementId: String,
    lowerOrder: String?,
    upperOrder: String?,
  ): Boolean {
    try {
      executeMutation(
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
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to move custom text replacement" }
      toast.show(ToastType.Error, MUTATION_FAILURE_MESSAGE)
      return false
    }
  }

  private suspend fun toggleTextReplacementState(
    item: NormalizedTextReplacement,
    enabled: Boolean,
  ) {
    if (item.state == desiredState(enabled)) return

    try {
      updateTextReplacementState(
        textReplacementId = item.textReplacementId,
        enabled = enabled,
      )
      query.refetch()
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to toggle text replacement state" }
      toast.show(ToastType.Error, MUTATION_FAILURE_MESSAGE)
    }
  }

  private suspend fun updateTextReplacementState(
    textReplacementId: String,
    enabled: Boolean,
  ) {
    executeMutation(
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
