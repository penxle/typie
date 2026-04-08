package co.typie.screen.home

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.HomeScreen_Query
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.text
import co.typie.graphql.type.buildDocument
import co.typie.graphql.type.buildEntity
import co.typie.graphql.type.buildFolder
import co.typie.graphql.type.buildSite
import co.typie.graphql.type.buildUser
import co.typie.service.SiteService
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class HomeViewModel(
  private val siteService: SiteService,
) : GraphQLViewModel() {
  val query =
    watchQuery(placeholderData()) { HomeScreen_Query(siteId = siteService.siteId) }
}

private fun placeholderData() = HomeScreen_Query.Data(PlaceholderResolver) {
  site = buildSite {
    name = ""
  }
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
            subtitle = if (it % 2 == 0) text(4..12) else null
            excerpt = text(20..30)
          }
        }
      }
    }
  }
}
