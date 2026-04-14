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
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildUser
import co.typie.graphql.fragment.NoteEntityPicker_entity
import co.typie.graphql.text
import co.typie.graphql.watchQuery
import co.typie.storage.Preference
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class NoteEntityPickerViewModel : ViewModel() {
  private val currentSiteId: String?
    get() = Preference.siteId

  var inputKeyword: String by mutableStateOf("")
    private set

  private var activeKeyword: String by mutableStateOf("")
    private set

  val recentQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = recentPlaceholderData(),
      skip = { currentSiteId == null },
    ) {
      NoteEntityPicker_Recent_Query(siteId = currentSiteId!!)
    }

  val searchQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      skip = { currentSiteId == null || activeKeyword.isBlank() },
      resetOnChange = false,
    ) {
      NoteEntityPicker_Search_Query(siteId = currentSiteId!!, query = activeKeyword)
    }

  val recentEntities: List<NoteEntityPicker_entity>
    get() = recentQuery.data.linkedEntities()

  val searchResults: List<NoteEntityPicker_entity>
    get() =
      searchQuery.data
        ?.search
        ?.hits
        ?.mapNotNull { it.linkedEntityOrNull() }
        ?.distinctBy { it.id }
        .orEmpty()

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
