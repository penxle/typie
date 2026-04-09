package co.typie.entity_transfer

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.touchlab.kermit.Logger
import co.typie.graphql.EntityClipboard_CopyEntities_Mutation
import co.typie.graphql.EntityClipboard_MoveEntities_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.executeMutation
import co.typie.graphql.type.CopyEntitiesInput
import co.typie.graphql.type.MoveEntitiesInput
import co.typie.overlay.Toast
import co.typie.service.SiteRefreshCoordinator
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.CancellationException
import org.koin.core.annotation.Single

private const val DEFAULT_PASTE_ERROR_MESSAGE = "붙여넣기 중 오류가 발생했어요"

private class EntityPasteException(
  val toastMessage: String,
  cause: Throwable,
) : Exception(cause)

enum class EntityClipboardMode { Copy, Cut }

data class EntityClipboardState(
  val mode: EntityClipboardMode,
  val sourceSiteId: String,
  val items: List<EntityTransferSource>,
)

data class EntityPasteTarget(
  val siteId: String,
  val destinationEntityId: String?,
  val destinationDepth: Int,
  val ancestorFolderIds: Set<String>,
  val lowerOrder: String?,
  val upperOrder: String?,
)

data class EntityClipboardCopyRequest(
  val entityIds: List<String>,
  val targetSiteId: String,
  val parentEntityId: String?,
  val lowerOrder: String?,
  val upperOrder: String?,
)

data class EntityClipboardMoveRequest(
  val entityIds: List<String>,
  val parentEntityId: String?,
  val lowerOrder: String?,
  val upperOrder: String?,
  val targetSiteId: String?,
)

interface EntityClipboardMutationExecutor {
  suspend fun copyEntities(request: EntityClipboardCopyRequest)

  suspend fun moveEntities(request: EntityClipboardMoveRequest)
}

@Single(binds = [EntityClipboardMutationExecutor::class])
class ApolloEntityClipboardMutationExecutor(
  private val apolloClient: ApolloClient,
) : EntityClipboardMutationExecutor {
  override suspend fun copyEntities(request: EntityClipboardCopyRequest) {
    apolloClient.executeMutation(
      EntityClipboard_CopyEntities_Mutation(
        input = CopyEntitiesInput.Builder()
          .entityIds(request.entityIds)
          .targetSiteId(request.targetSiteId)
          .parentEntityId(request.parentEntityId)
          .lowerOrder(request.lowerOrder)
          .upperOrder(request.upperOrder)
          .build(),
      ),
    )
  }

  override suspend fun moveEntities(request: EntityClipboardMoveRequest) {
    apolloClient.executeMutation(
      EntityClipboard_MoveEntities_Mutation(
        input = MoveEntitiesInput.Builder()
          .entityIds(request.entityIds)
          .parentEntityId(request.parentEntityId)
          .lowerOrder(request.lowerOrder)
          .upperOrder(request.upperOrder)
          .targetSiteId(request.targetSiteId)
          .build(),
      ),
    )
  }
}

@Single
class EntityClipboardService(
  private val executor: EntityClipboardMutationExecutor,
  private val toast: Toast,
  private val siteRefreshCoordinator: SiteRefreshCoordinator,
) {
  var state: EntityClipboardState? by mutableStateOf(null)
    private set

  val currentState: EntityClipboardState?
    get() = state

  fun setCopy(
    sourceSiteId: String,
    items: List<EntityTransferSource>,
  ) {
    setState(
      mode = EntityClipboardMode.Copy,
      sourceSiteId = sourceSiteId,
      items = items,
    )
  }

  fun setCut(
    sourceSiteId: String,
    items: List<EntityTransferSource>,
  ) {
    setState(
      mode = EntityClipboardMode.Cut,
      sourceSiteId = sourceSiteId,
      items = items,
    )
  }

  fun clear() {
    state = null
  }

  fun canPaste(target: EntityPasteTarget): Boolean {
    val clipboard = state ?: return false
    return clipboard.items.all { item ->
      when (item) {
        is EntityTransferSource.Document -> true
        is EntityTransferSource.Folder -> {
          target.destinationEntityId != item.id &&
            item.id !in target.ancestorFolderIds &&
            item.canTransferIntoDestinationDepth(target.destinationDepth)
        }
      }
    }
  }

  suspend fun pasteInto(target: EntityPasteTarget): Boolean {
    val clipboard = state ?: return false
    if (!canPaste(target)) {
      return false
    }

    val itemIds = clipboard.items.map(EntityTransferSource::id)
    val itemCount = itemIds.size

    try {
      toast.withLoading(
        message = "${itemCount}개의 항목을 붙여넣는 중이에요",
        errorMessage = DEFAULT_PASTE_ERROR_MESSAGE,
      ) {
        try {
          when (clipboard.mode) {
            EntityClipboardMode.Copy -> executor.copyEntities(
              EntityClipboardCopyRequest(
                entityIds = itemIds,
                targetSiteId = target.siteId,
                parentEntityId = target.destinationEntityId,
                lowerOrder = target.lowerOrder,
                upperOrder = target.upperOrder,
              ),
            )

            EntityClipboardMode.Cut -> executor.moveEntities(
              EntityClipboardMoveRequest(
                entityIds = itemIds,
                parentEntityId = target.destinationEntityId,
                lowerOrder = target.lowerOrder,
                upperOrder = target.upperOrder,
                targetSiteId = target.siteId.takeIf { it != clipboard.sourceSiteId },
              ),
            )
          }
        } catch (e: CancellationException) {
          throw e
        } catch (e: Throwable) {
          throw EntityPasteException(
            toastMessage = getPasteErrorMessage(e),
            cause = e,
          )
        }

        success("${itemCount}개의 항목을 붙여넣었어요")
      }
    } catch (e: EntityPasteException) {
      toast.show(type = co.typie.overlay.ToastType.Error, message = e.toastMessage)
      Logger.e(e) { "Failed to paste ${itemIds.size} entities" }
      return false
    } catch (e: CancellationException) {
      throw e
    } catch (e: Exception) {
      Logger.e(e) { "Failed to paste ${itemIds.size} entities" }
      return false
    }

    if (clipboard.mode == EntityClipboardMode.Cut) {
      clear()
    }

    siteRefreshCoordinator.notifySiteChanged(target.siteId)
    if (clipboard.sourceSiteId != target.siteId) {
      siteRefreshCoordinator.notifySiteChanged(clipboard.sourceSiteId)
    }
    return true
  }

  private fun setState(
    mode: EntityClipboardMode,
    sourceSiteId: String,
    items: List<EntityTransferSource>,
  ) {
    state = if (sourceSiteId.isBlank() || items.isEmpty()) {
      null
    } else {
      EntityClipboardState(
        mode = mode,
        sourceSiteId = sourceSiteId,
        items = items,
      )
    }
  }
}

fun getPasteErrorMessage(error: Throwable): String {
  val typieError = error as? TypieError ?: return DEFAULT_PASTE_ERROR_MESSAGE
  return when (typieError.code) {
    "site_mismatch" -> "이 위치에는 붙여넣을 수 없어요."
    "circular_reference" -> "자기 자신 또는 하위 항목 안에는 붙여넣을 수 없어요."
    "paste_source_not_found" -> "붙여넣을 항목을 찾을 수 없어요."
    "character_count_limit_exceeded" -> "현재 플랜의 글자 수 제한을 초과했어요."
    "blob_size_limit_exceeded" -> "현재 플랜의 파일 크기 제한을 초과했어요."
    else -> DEFAULT_PASTE_ERROR_MESSAGE
  }
}
