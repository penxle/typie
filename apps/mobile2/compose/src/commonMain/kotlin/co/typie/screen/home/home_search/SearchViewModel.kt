package co.typie.screen.home.home_search

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.HomeScreen_Search_Query
import co.typie.graphql.HomeSearch_Header_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildSite
import co.typie.graphql.watchQuery
import co.typie.storage.Preference
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class SearchViewModel : ViewModel() {
  var shouldAnimateHeaderOnEnter by mutableStateOf(true)
    private set

  var query by mutableStateOf("")
    private set

  val recentSearches = Preference.recentSearches

  var activeQuery by mutableStateOf("")
    private set

  val siteQuery =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderSiteData(),
      skip = { Preference.siteId.value == null },
    ) {
      HomeSearch_Header_Query(siteId = Preference.siteId.value!!)
    }

  val searchResults =
    Apollo.watchQuery(
      scope = viewModelScope,
      skip = { activeQuery.isBlank() || Preference.siteId.value == null },
      resetOnChange = false,
    ) {
      HomeScreen_Search_Query(siteId = Preference.siteId.value!!, query = activeQuery)
    }

  private var debounceJob: Job? = null

  fun updateQuery(value: String) {
    query = value
    debounceJob?.cancel()
    if (value.isBlank()) {
      activeQuery = ""
      return
    }
    debounceJob = viewModelScope.launch {
      delay(300)
      activeQuery = value
    }
  }

  fun submitQuery() {
    val q = query.trim()
    if (q.isBlank()) return
    debounceJob?.cancel()
    activeQuery = q
  }

  fun saveRecentSearch(queryText: String) {
    val trimmed = queryText.trim()
    if (trimmed.isBlank()) return
    val updated =
      Preference.recentSearches.value.toMutableList().apply {
        remove(trimmed)
        add(0, trimmed)
        if (size > 10) removeLast()
      }
    Preference.recentSearches.value = updated
  }

  fun removeRecentSearch(queryText: String) {
    Preference.recentSearches.value = Preference.recentSearches.value - queryText
  }

  fun onHeaderEnterAnimationConsumed() {
    shouldAnimateHeaderOnEnter = false
  }
}

private fun placeholderSiteData() =
  HomeSearch_Header_Query.Data(PlaceholderResolver) { site = buildSite { name = "" } }
