package co.typie.entity_transfer

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import co.typie.graphql.Apollo
import co.typie.graphql.EntityClipboard_CopyEntities_Mutation
import co.typie.graphql.EntityClipboard_MoveEntities_Mutation
import co.typie.graphql.TypieError
import co.typie.graphql.executeMutation
import co.typie.graphql.type.CopyEntitiesInput
import co.typie.graphql.type.MoveEntitiesInput
import co.typie.result.Task
import co.typie.result.task
import co.typie.service.SiteRefreshCoordinator
import kotlinx.coroutines.CancellationException

enum class EntityClipboardMode {
  Copy,
  Cut,
}

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

sealed interface PasteError {
  data object SiteMismatch : PasteError

  data object CircularReference : PasteError

  data object SourceNotFound : PasteError

  data object CharacterCountLimitExceeded : PasteError

  data object BlobSizeLimitExceeded : PasteError
}

interface EntityClipboardMutationExecutor {
  suspend fun copyEntities(request: EntityClipboardCopyRequest)

  suspend fun moveEntities(request: EntityClipboardMoveRequest)
}

object ApolloEntityClipboardMutationExecutor : EntityClipboardMutationExecutor {
  override suspend fun copyEntities(request: EntityClipboardCopyRequest) {
    Apollo.executeMutation(
      EntityClipboard_CopyEntities_Mutation(
        input =
          CopyEntitiesInput.Builder()
            .entityIds(request.entityIds)
            .targetSiteId(request.targetSiteId)
            .parentEntityId(request.parentEntityId)
            .lowerOrder(request.lowerOrder)
            .upperOrder(request.upperOrder)
            .build()
      )
    )
  }

  override suspend fun moveEntities(request: EntityClipboardMoveRequest) {
    Apollo.executeMutation(
      EntityClipboard_MoveEntities_Mutation(
        input =
          MoveEntitiesInput.Builder()
            .entityIds(request.entityIds)
            .parentEntityId(request.parentEntityId)
            .lowerOrder(request.lowerOrder)
            .upperOrder(request.upperOrder)
            .targetSiteId(request.targetSiteId)
            .build()
      )
    )
  }
}

object EntityClipboardService {
  var state: EntityClipboardState? by mutableStateOf(null)
    private set

  val currentState: EntityClipboardState?
    get() = state

  fun setCopy(sourceSiteId: String, items: List<EntityTransferSource>) {
    setState(mode = EntityClipboardMode.Copy, sourceSiteId = sourceSiteId, items = items)
  }

  fun setCut(sourceSiteId: String, items: List<EntityTransferSource>) {
    setState(mode = EntityClipboardMode.Cut, sourceSiteId = sourceSiteId, items = items)
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

  fun pasteInto(target: EntityPasteTarget): Task<Int, Int, PasteError> = task {
    val clipboard = state ?: return@task 0
    if (!canPaste(target)) return@task 0

    val itemIds = clipboard.items.map(EntityTransferSource::id)
    emit(itemIds.size)

    try {
      when (clipboard.mode) {
        EntityClipboardMode.Copy ->
          ApolloEntityClipboardMutationExecutor.copyEntities(
            EntityClipboardCopyRequest(
              entityIds = itemIds,
              targetSiteId = target.siteId,
              parentEntityId = target.destinationEntityId,
              lowerOrder = target.lowerOrder,
              upperOrder = target.upperOrder,
            )
          )

        EntityClipboardMode.Cut ->
          ApolloEntityClipboardMutationExecutor.moveEntities(
            EntityClipboardMoveRequest(
              entityIds = itemIds,
              parentEntityId = target.destinationEntityId,
              lowerOrder = target.lowerOrder,
              upperOrder = target.upperOrder,
              targetSiteId = target.siteId.takeIf { it != clipboard.sourceSiteId },
            )
          )
      }
    } catch (e: CancellationException) {
      throw e
    } catch (e: Throwable) {
      val pasteError = classifyPasteError(e)
      if (pasteError != null) raise(pasteError)
      throw e
    }

    if (clipboard.mode == EntityClipboardMode.Cut) {
      clear()
    }

    SiteRefreshCoordinator.notifySiteChanged(target.siteId)
    if (clipboard.sourceSiteId != target.siteId) {
      SiteRefreshCoordinator.notifySiteChanged(clipboard.sourceSiteId)
    }

    itemIds.size
  }

  private fun setState(
    mode: EntityClipboardMode,
    sourceSiteId: String,
    items: List<EntityTransferSource>,
  ) {
    state =
      if (sourceSiteId.isBlank() || items.isEmpty()) {
        null
      } else {
        EntityClipboardState(mode = mode, sourceSiteId = sourceSiteId, items = items)
      }
  }
}

private fun classifyPasteError(error: Throwable): PasteError? {
  val typieError = error as? TypieError ?: return null
  return when (typieError.code) {
    "site_mismatch" -> PasteError.SiteMismatch
    "circular_reference" -> PasteError.CircularReference
    "paste_source_not_found" -> PasteError.SourceNotFound
    "character_count_limit_exceeded" -> PasteError.CharacterCountLimitExceeded
    "blob_size_limit_exceeded" -> PasteError.BlobSizeLimitExceeded
    else -> null
  }
}
