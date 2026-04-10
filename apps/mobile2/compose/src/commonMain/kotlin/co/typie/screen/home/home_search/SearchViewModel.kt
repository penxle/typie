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
import co.typie.service.SiteService
import co.typie.storage.Prefs
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class SearchViewModel : ViewModel() {
  var shouldAnimateHeaderOnEnter by mutableStateOf(true)
    private set

  var query by mutableStateOf("")
    private set

  var recentSearches by mutableStateOf<List<String>>(emptyList())
    private set

  var activeQuery by mutableStateOf("")
    private set

  val siteQuery =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderSiteData()) {
      HomeSearch_Header_Query(siteId = SiteService.siteId)
    }

  val searchResults =
    Apollo.watchQuery(
      scope = viewModelScope,
      skip = { activeQuery.isBlank() },
      resetOnChange = false,
    ) {
      HomeScreen_Search_Query(siteId = SiteService.siteId, query = activeQuery)
    }

  private var storedRecentSearches: List<String> by Prefs("recent_searches", emptyList())
  private var debounceJob: Job? = null

  init {
    recentSearches = storedRecentSearches
  }

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
      recentSearches.toMutableList().apply {
        remove(trimmed)
        add(0, trimmed)
        if (size > 10) removeLast()
      }
    recentSearches = updated
    storedRecentSearches = updated
  }

  fun removeRecentSearch(queryText: String) {
    val updated = recentSearches - queryText
    recentSearches = updated
    storedRecentSearches = updated
  }

  fun onHeaderEnterAnimationConsumed() {
    shouldAnimateHeaderOnEnter = false
  }
}

private fun placeholderSiteData() =
  HomeSearch_Header_Query.Data(PlaceholderResolver) { site = buildSite { name = "" } }
