package co.typie.domain.note

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.NoteEntityPicker_Recent_Query
import co.typie.graphql.NoteEntityPicker_Search_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.QueryState
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildUser
import co.typie.graphql.fragment.NoteEntityPicker_entity
import co.typie.graphql.text
import co.typie.graphql.watchQuery
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

internal class NoteEntityPickerViewModel(private val siteId: String) : ViewModel() {
  private val recentPlaceholder = recentPlaceholderData()

  var inputKeyword: String by mutableStateOf("")
    private set

  private var activeKeyword: String by mutableStateOf("")
    private set

  val recentQuery =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = recentPlaceholder) {
      NoteEntityPicker_Recent_Query(siteId = siteId)
    }

  val searchQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      skip = { activeKeyword.isBlank() },
      resetOnChange = false,
    ) {
      NoteEntityPicker_Search_Query(siteId = siteId, query = activeKeyword)
    }

  val recentEntities: List<NoteEntityPicker_entity>
    get() = (recentQuery.state as? QueryState.Success)?.data?.linkedEntities().orEmpty()

  val recentPlaceholderEntities: List<NoteEntityPicker_entity>
    get() = recentPlaceholder.linkedEntities()

  val searchState: QueryState<List<NoteEntityPicker_Search_Query.Hit>>
    get() {
      val input = inputKeyword
      if (
        input.isBlank() ||
          activeKeyword != input ||
          searchQuery.stateQuery != NoteEntityPicker_Search_Query(siteId = siteId, query = input)
      ) {
        return QueryState.Loading
      }

      return when (val state = searchQuery.state) {
        QueryState.Loading -> QueryState.Loading
        is QueryState.Success -> QueryState.Success(state.data.search.hits)
        is QueryState.Error -> state
      }
    }

  private var debounceJob: Job? = null

  fun setKeyword(value: String) {
    inputKeyword = value
    debounceJob?.cancel()

    if (value.isBlank()) {
      activeKeyword = ""
      return
    }

    debounceJob = viewModelScope.launch {
      delay(300)
      activeKeyword = value
    }
  }

  fun clearSearch() {
    debounceJob?.cancel()
    inputKeyword = ""
    activeKeyword = ""
  }
}

private fun recentPlaceholderData() =
  NoteEntityPicker_Recent_Query.Data(PlaceholderResolver) {
    me = buildUser {
      recentlyViewedEntities =
        List(5) { index ->
          buildEntity {
            id = "placeholder-recent-$index"
            slug = "placeholder-recent-$index"
            icon = "file"
            iconColor = "gray"
            node = buildDocument {
              id = "placeholder-recent-document-$index"
              title = text(6..12)
              subtitle = null
              excerpt = text(14..22)
              entity = buildEntity { id = "placeholder-recent-document-entity-$index" }
            }
          }
        }
    }
  }
