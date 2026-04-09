package co.typie.screen.space

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.SpaceScreen_Query
import co.typie.service.SiteService
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class SpaceViewModel(
  private val siteService: SiteService,
) : GraphQLViewModel() {
  val query = watchQuery { SpaceScreen_Query(siteId = siteService.siteId) }
}
