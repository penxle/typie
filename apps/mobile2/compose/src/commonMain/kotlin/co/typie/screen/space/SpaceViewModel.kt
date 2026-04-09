package co.typie.screen.space

import co.touchlab.kermit.Logger
import co.typie.graphql.EntityContainer_MoveEntity_Mutation
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.SpaceScreen_Query
import co.typie.graphql.type.MoveEntityInput
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.service.SiteService
import kotlinx.coroutines.CancellationException
import org.koin.core.annotation.KoinViewModel

private const val GENERIC_MUTATION_ERROR_MESSAGE = "오류가 발생했어요. 잠시 후 다시 시도해주세요."

@KoinViewModel
class SpaceViewModel(
  private val siteService: SiteService,
  private val toast: Toast,
) : GraphQLViewModel() {
  private var hasEnteredScreen = false

  val siteId: String
    get() = siteService.siteId

  val query = watchQuery { SpaceScreen_Query(siteId = siteService.siteId) }

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
  ): Boolean {
    try {
      executeMutation(
        EntityContainer_MoveEntity_Mutation(
          input = MoveEntityInput.Builder()
            .entityId(entityId)
            .apply {
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

      refetch()
      return true
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to move root entity" }
      toast.show(ToastType.Error, GENERIC_MUTATION_ERROR_MESSAGE)
      return false
    }
  }
}
