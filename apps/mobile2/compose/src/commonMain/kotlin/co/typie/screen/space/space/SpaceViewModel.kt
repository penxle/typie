package co.typie.screen.space.space

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.graphql.EntityContainer_MoveEntity_Mutation
import co.typie.graphql.SpaceScreen_Query
import co.typie.graphql.executeMutation
import co.typie.graphql.type.MoveEntityInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import co.typie.service.SiteService
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class SpaceViewModel(
  private val apolloClient: ApolloClient,
  private val siteService: SiteService,
) : ViewModel() {
  private var hasEnteredScreen = false

  val siteId: String
    get() = siteService.siteId

  val query = apolloClient.watchQuery(scope = viewModelScope) { SpaceScreen_Query(siteId = siteService.siteId) }

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

  suspend fun moveRootEntity(
    entityId: String,
    lowerOrder: String?,
    upperOrder: String?,
  ): Result<Unit, Nothing> = result {
    apolloClient.executeMutation(
      EntityContainer_MoveEntity_Mutation(
        input = MoveEntityInput.Builder()
          .entityId(entityId)
          .apply {
            if (lowerOrder != null) lowerOrder(lowerOrder)
            if (upperOrder != null) upperOrder(upperOrder)
          }
          .build(),
      ),
    )
    refetch()
  }
}
