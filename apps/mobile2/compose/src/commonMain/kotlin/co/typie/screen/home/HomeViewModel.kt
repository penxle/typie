package co.typie.screen.home

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.HomeScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.text
import co.typie.graphql.type.buildDocument
import co.typie.graphql.type.buildEntity
import co.typie.graphql.type.buildFolder
import co.typie.graphql.type.buildUser
import co.typie.service.SiteService
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class HomeViewModel(
  private val siteService: SiteService,
) : GraphQLViewModel() {
  val query =
    watchQuery(placeholderData()) { HomeScreen_Query(siteId = siteService.siteId) }

  var searching by mutableStateOf(false)
}

private fun placeholderData() = HomeScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    recentlyViewedEntities = List(15) {
      buildEntity {
        node = if (it < 5) {
          buildFolder {
            name = text(5..10)
          }
        } else {
          buildDocument {
            title = text(5..20)
            excerpt = text(20..30)
          }
        }
      }
    }
  }
}
