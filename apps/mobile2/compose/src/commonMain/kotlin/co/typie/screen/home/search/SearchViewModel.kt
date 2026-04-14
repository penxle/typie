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

  var inputKeyword by mutableStateOf("")
    private set

  private var activeKeyword by mutableStateOf("")

  private var debounceJob: Job? = null

  fun setKeyword(keyword: String) {
    inputKeyword = keyword
    debounceJob?.cancel()
    debounceJob = viewModelScope.launch {
      delay(300)
      activeKeyword = keyword
    }
  }

  fun flush() {
    debounceJob?.cancel()
    activeKeyword = ""
  }

  fun addRecent() {
    Preference.recentSearches =
      Preference.recentSearches.toMutableList().apply {
        remove(activeKeyword)
        add(0, activeKeyword)
        take(10)
      }
  }

  fun removeRecent(keyword: String) {
    Preference.recentSearches -= keyword
  }
}

private fun placeholderData() =
  SearchScreen_Query.Data(PlaceholderResolver) { site = buildSite { name = text(5..10) } }
