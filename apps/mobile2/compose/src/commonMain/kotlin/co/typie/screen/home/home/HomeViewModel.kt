package co.typie.screen.home.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.Apollo
import co.typie.graphql.HomeScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildDocument
import co.typie.graphql.builder.buildEntity
import co.typie.graphql.builder.buildFolder
import co.typie.graphql.builder.buildSite
import co.typie.graphql.builder.buildUser
import co.typie.graphql.text
import co.typie.graphql.watchQuery
import co.typie.storage.Preference

class HomeViewModel : ViewModel() {
  private var hasEnteredScreen = false

  val siteId: String?
    get() = Preference.siteId.value

  val query =
    Apollo.watchQuery(
      scope = viewModelScope,
      placeholderData = placeholderData(),
      skip = { Preference.siteId.value == null },
    ) {
      HomeScreen_Query(siteId = Preference.siteId.value!!)
    }

  fun refetch() {
    query.refetch()
  }

  fun onScreenEntered() {
    if (hasEnteredScreen) {
      refetch()
      return
    }

    hasEnteredScreen = true
  }
}

private fun placeholderData() =
  HomeScreen_Query.Data(PlaceholderResolver) {
    site = buildSite { name = "" }
    me = buildUser {
      recentlyViewedEntities =
        List(15) {
          buildEntity {
            node =
              if (it < 5) {
                buildFolder { name = text(5..10) }
              } else {
                buildDocument {
                  title = text(5..20)
                  subtitle = if (it % 2 == 0) text(4..12) else null
                  excerpt = text(20..30)
                }
              }
          }
        }
    }
  }
