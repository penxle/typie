package co.typie.screen.space

import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.SpaceScreen_Query
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class SpaceViewModel : GraphQLViewModel() {
  val query = watchQuery { SpaceScreen_Query() }
}
