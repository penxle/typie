package co.typie.overlay

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.entity_transfer.EntityTransferSource
import co.typie.graphql.EntityContainer_MoveEntity_Mutation
import co.typie.graphql.EntityMoveSheet_Folder_Query
import co.typie.graphql.EntityMoveSheet_Root_Query
import co.typie.graphql.executeMutation
import co.typie.graphql.type.MoveEntityInput
import co.typie.graphql.watchQuery
import co.typie.result.Result
import co.typie.result.result
import co.typie.service.SiteService
import com.apollographql.apollo.ApolloClient
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class EntityMoveSheetViewModel(
  private val apolloClient: ApolloClient,
  private val siteService: SiteService,
) : ViewModel() {
  var destinationEntityId: String? by mutableStateOf(null)
    private set

  val rootQuery = apolloClient.watchQuery(
    scope = viewModelScope,
    skip = { destinationEntityId != null },
  ) {
    EntityMoveSheet_Root_Query(siteId = siteService.siteId)
  }

  val entityQuery = apolloClient.watchQuery(
    scope = viewModelScope,
    skip = { destinationEntityId == null },
  ) {
    EntityMoveSheet_Folder_Query(entityId = requireNotNull(destinationEntityId))
  }

  fun showRoot() {
    destinationEntityId = null
  }

  fun showDestination(entityId: String?) {
    destinationEntityId = entityId
  }

  fun refetch() {
    if (destinationEntityId == null) {
      rootQuery.refetch()
    } else {
      entityQuery.refetch()
    }
  }

  suspend fun moveEntity(
    source: EntityTransferSource,
    parentEntityId: String?,
    lowerOrder: String?,
    upperOrder: String?,
  ): Result<Unit, Nothing> = result {
    apolloClient.executeMutation(
      EntityContainer_MoveEntity_Mutation(
        input = MoveEntityInput.Builder()
          .entityId(source.id)
          .parentEntityId(parentEntityId)
          .apply {
            if (parentEntityId == null) treatEmptyParentIdAsRoot(true)
            if (lowerOrder != null) lowerOrder(lowerOrder)
            if (upperOrder != null) upperOrder(upperOrder)
          }
          .build(),
      ),
    )
  }
}
