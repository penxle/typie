package co.typie.screen.settings.textreplacements

import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.form.FormState
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.TextReplacementsScreen_CreateTextReplacement_Mutation
import co.typie.graphql.TextReplacementsScreen_DeleteTextReplacement_Mutation
import co.typie.graphql.TextReplacementsScreen_MoveTextReplacement_Mutation
import co.typie.graphql.TextReplacementsScreen_Query
import co.typie.graphql.TextReplacementsScreen_UpdateTextReplacement_Mutation
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildTextReplacement
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.midpointOrder
import co.typie.graphql.type.CreateTextReplacementInput
import co.typie.graphql.type.DeleteTextReplacementInput
import co.typie.graphql.type.MoveTextReplacementInput
import co.typie.graphql.type.TextReplacementState
import co.typie.graphql.type.UpdateTextReplacementInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import com.apollographql.apollo.api.Optional
import kotlinx.coroutines.CoroutineScope

class TextReplacementForm(scope: CoroutineScope, textReplacement: CustomTextReplacement?) :
  FormState(scope) {
  val match = field(textReplacement?.match.orEmpty()) { required("찾을 텍스트를 입력해주세요.") }
  val substitute = field(textReplacement?.substitute.orEmpty()) { required("삽입할 텍스트를 입력해주세요.") }
  val note = field(textReplacement?.note.orEmpty())
  val regex = field(textReplacement?.regex ?: false) { focusable = false }

  init {
    validate {
      check(substitute, match.value != substitute.value) { "찾을 텍스트와 삽입할 텍스트가 같아요." }
      check(match, !regex.value || isValidRegex(match.value)) { "정규식이 올바르지 않아요." }
    }
  }
}

class TextReplacementsViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      TextReplacementsScreen_Query()
    }

  val smartQuotes by derivedStateOf {
    val byId = query.data.me.textReplacements.associateBy { it.textReplacementId }
    SMART_QUOTE_IDS.mapNotNull(byId::get)
  }

  val presets by derivedStateOf {
    query.data.me.textReplacements
      .filter { it.isPreset && !it.isSmartQuote }
      .sortedBy { it.order.orEmpty() }
  }

  val customs by derivedStateOf {
    query.data.me.textReplacements.filter { !it.isPreset }.sortedBy { it.order.orEmpty() }
  }

  suspend fun createTextReplacement(
    match: String,
    substitute: String,
    regex: Boolean,
    note: String?,
  ): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      TextReplacementsScreen_CreateTextReplacement_Mutation(
        input =
          CreateTextReplacementInput(
            match = match,
            substitute = substitute,
            regex = Optional.present(regex),
            note = Optional.presentIfNotNull(note),
            lowerOrder = Optional.presentIfNotNull(customs.lastOrNull()?.order),
          )
      )
    )

    query.refetch()
  }

  suspend fun updateTextReplacement(
    id: String,
    match: String,
    substitute: String,
    regex: Boolean,
    note: String?,
  ): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      TextReplacementsScreen_UpdateTextReplacement_Mutation(
        input =
          UpdateTextReplacementInput(
            textReplacementId = id,
            match = Optional.present(match),
            substitute = Optional.present(substitute),
            regex = Optional.present(regex),
            note = Optional.presentIfNotNull(note),
          )
      )
    )

    query.refetch()
  }

  suspend fun updateTextReplacementState(id: String, enabled: Boolean): Result<Unit, Nothing> =
    result {
      Apollo.executeMutation(
        TextReplacementsScreen_UpdateTextReplacement_Mutation(
          input =
            UpdateTextReplacementInput(
              textReplacementId = id,
              state =
                Optional.present(
                  if (enabled) TextReplacementState.ACTIVE else TextReplacementState.DISABLED
                ),
            )
        )
      )

      query.refetch()
    }

  suspend fun deleteTextReplacement(id: String): Result<Unit, Nothing> = result {
    Apollo.executeMutation(
      TextReplacementsScreen_DeleteTextReplacement_Mutation(
        input = DeleteTextReplacementInput(textReplacementId = id)
      )
    )

    query.refetch()
  }

  suspend fun updateSmartQuotesTextReplacementState(enabled: Boolean): Result<Unit, Nothing> =
    result {
      for (id in SMART_QUOTE_IDS) {
        updateTextReplacementState(id, enabled).unwrap()
      }
    }

  suspend fun reorderCustom(movedKey: String, orderedKeys: List<String>): Result<Unit, Nothing> =
    result {
      val serverOrder = customs.map { it.textReplacementId }
      if (orderedKeys == serverOrder) return@result

      val bounds = neighboringOrders(orderedKeys, movedKey, customs) ?: return@result
      val newOrder = midpointOrder(bounds.first, bounds.second)

      val builder = MoveTextReplacementInput.Builder().textReplacementId(movedKey)
      bounds.first?.let { builder.lowerOrder(it) }
      bounds.second?.let { builder.upperOrder(it) }

      Apollo.executeMutation(
        TextReplacementsScreen_MoveTextReplacement_Mutation(input = builder.build()),
        optimisticUpdate =
          TextReplacementsScreen_MoveTextReplacement_Mutation.Data(PlaceholderResolver) {
            moveTextReplacement = buildTextReplacement {
              id = movedKey
              order = newOrder
            }
          },
      )
    }
}

private fun placeholderData() =
  TextReplacementsScreen_Query.Data(PlaceholderResolver) {
    me = buildUser { textReplacements = emptyList() }
  }
