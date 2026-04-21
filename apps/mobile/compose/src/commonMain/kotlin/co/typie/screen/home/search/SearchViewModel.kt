package co.typie.screen.home.search

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.SearchScreen_Query
import co.typie.graphql.SearchScreen_Search_Query
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildSite
import co.typie.graphql.text
import co.typie.graphql.watchQuery
import co.typie.storage.Preference
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class SearchViewModel : ViewModel() {
  var inputKeyword by mutableStateOf("")
    private set

  private var activeKeyword by mutableStateOf("")

  private var debounceJob: Job? = null

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      skip = { Preference.siteId == null },
    ) {
      SearchScreen_Query(siteId = Preference.siteId!!)
    }

  val searchQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      skip = { activeKeyword.isBlank() || Preference.siteId == null },
      resetOnChange = false,
    ) {
      SearchScreen_Search_Query(siteId = Preference.siteId!!, query = activeKeyword)
    }

  fun setKeyword(keyword: String) {
    inputKeyword = keyword
    debounceJob?.cancel()

    if (keyword.isBlank()) {
      activeKeyword = ""
      return
    }

    debounceJob = viewModelScope.launch {
      delay(300)
      activeKeyword = keyword
    }
  }

  fun submitKeyword(keyword: String = inputKeyword) {
    debounceJob?.cancel()
    inputKeyword = keyword
    activeKeyword = keyword
  }

  fun addRecent() {
    Preference.recentSearches = updatedRecentSearches(Preference.recentSearches, activeKeyword)
  }

  fun removeRecent(keyword: String) {
    Preference.recentSearches -= keyword
  }
}

internal fun updatedRecentSearches(recentSearches: List<String>, keyword: String): List<String> {
  if (keyword.isBlank()) return recentSearches

  return buildList {
      add(keyword)
      recentSearches.filterNotTo(this) { it == keyword }
    }
    .take(10)
}

private fun placeholderData() =
  SearchScreen_Query.Data(PlaceholderResolver) { site = buildSite { name = text(5..10) } }
