package co.typie.screen.entity_move

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.touchlab.kermit.Logger
import co.typie.graphql.EntityContainer_MoveEntity_Mutation
import co.typie.graphql.EntityMoveSheet_Folder_Query
import co.typie.graphql.EntityMoveSheet_Root_Query
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.type.MoveEntityInput
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.service.SiteService
import kotlinx.coroutines.CancellationException
import org.koin.core.annotation.KoinViewModel

private const val GENERIC_MUTATION_ERROR_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."

@KoinViewModel
class EntityMoveSheetViewModel(
  private val siteService: SiteService,
  private val toast: Toast,
) : GraphQLViewModel() {
  var destinationEntityId: String? by mutableStateOf(null)
    private set

  val rootQuery = watchQuery(
    skip = { destinationEntityId != null },
  ) {
    EntityMoveSheet_Root_Query(siteId = siteService.siteId)
  }

  val entityQuery = watchQuery(
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
    source: MoveSourceEntity,
    parentEntityId: String?,
    lowerOrder: String?,
    upperOrder: String?,
  ): Boolean {
    try {
      executeMutation(
        EntityContainer_MoveEntity_Mutation(
          input = MoveEntityInput.Builder()
            .entityId(source.id)
            .parentEntityId(parentEntityId)
            .apply {
              if (parentEntityId == null) {
                treatEmptyParentIdAsRoot(true)
              }
              if (lowerOrder != null) {
                lowerOrder(lowerOrder)
              }
              if (upperOrder != null) {
                upperOrder(upperOrder)
              }
            }
            .build(),
        ),
      )
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to move entity ${source.id}" }
      toast.show(ToastType.Error, GENERIC_MUTATION_ERROR_MESSAGE)
      return false
    }
  }
}
