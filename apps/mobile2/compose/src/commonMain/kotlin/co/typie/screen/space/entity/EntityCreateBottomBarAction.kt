package co.typie.screen.space.entity

import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import co.typie.overlay.LocalToast
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.shell.SpaceBottomBarActionButton
import kotlinx.coroutines.launch

@Composable
fun EntityCreateBottomBarAction(
  model: EntityCreateViewModel,
  siteId: String?,
  parentEntityId: String? = null,
  onCreated: () -> Unit = {},
  onFolderCreated: suspend (String) -> Unit,
  onDocumentCreated: suspend (String) -> Unit,
) {
  val toast = LocalToast.current
  val presenterScope = rememberCoroutineScope()

  SpaceBottomBarActionButton(
    onCreateFolder = {
      if (model.isCreating) return@SpaceBottomBarActionButton
      val resolvedSiteId = siteId ?: return@SpaceBottomBarActionButton
      presenterScope.launch {
        model
          .createFolder(siteId = resolvedSiteId, parentEntityId = parentEntityId)
          .withDefaultExceptionHandler(toast)
          .onOk { createdEntityId ->
            onCreated()
            onFolderCreated(createdEntityId)
          }
      }
    },
    onCreateDocument = {
      if (model.isCreating) return@SpaceBottomBarActionButton
      val resolvedSiteId = siteId ?: return@SpaceBottomBarActionButton
      presenterScope.launch {
        model
          .createDocument(siteId = resolvedSiteId, parentEntityId = parentEntityId)
          .withDefaultExceptionHandler(toast)
          .onOk { createdSlug ->
            onCreated()
            onDocumentCreated(createdSlug)
          }
      }
    },
  )
}
