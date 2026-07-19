package co.typie.screen.space.entity

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import co.typie.domain.subscription.GatedAction
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.gate
import co.typie.icons.Lucide
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.bottombar.ActionMenuItem
import co.typie.ui.component.bottombar.BottomBarAction
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.LocalToast
import kotlinx.coroutines.launch

@Composable
fun rememberEntityCreateBottomBarAction(
  model: EntityCreateViewModel,
  siteId: String?,
  parentEntityId: String? = null,
  onCreated: () -> Unit = {},
  onFolderCreated: suspend (String) -> Unit,
  onDocumentCreated: suspend (String) -> Unit,
): BottomBarAction {
  val toast = LocalToast.current
  val sheet = LocalSheet.current
  val presenterScope = rememberCoroutineScope()
  val onCreatedRef by rememberUpdatedState(onCreated)
  val onFolderCreatedRef by rememberUpdatedState(onFolderCreated)
  val onDocumentCreatedRef by rememberUpdatedState(onDocumentCreated)

  return remember(model, siteId, parentEntityId, presenterScope, toast, sheet) {
    BottomBarAction(
      icon = Lucide.SquarePlus,
      menus =
        listOf(
          ActionMenuItem(
            icon = Lucide.FolderPlus,
            label = "여기에 폴더 만들기",
            onClick = {
              if (model.isCreating) return@ActionMenuItem
              val resolvedSiteId = siteId ?: return@ActionMenuItem
              presenterScope.launch {
                if (!SubscriptionService.gate(sheet, GatedAction.CreateFolder)) return@launch
                model
                  .createFolder(siteId = resolvedSiteId, parentEntityId = parentEntityId)
                  .withDefaultExceptionHandler(toast)
                  .onOk { createdEntityId ->
                    onCreatedRef()
                    onFolderCreatedRef(createdEntityId)
                  }
              }
            },
          ),
          ActionMenuItem(
            icon = Lucide.SquarePen,
            label = "여기에 문서 만들기",
            onClick = {
              if (model.isCreating) return@ActionMenuItem
              val resolvedSiteId = siteId ?: return@ActionMenuItem
              presenterScope.launch {
                if (!SubscriptionService.gate(sheet, GatedAction.CreateDocument)) return@launch
                model
                  .createDocument(siteId = resolvedSiteId, parentEntityId = parentEntityId)
                  .withDefaultExceptionHandler(toast)
                  .onOk { createdEntityId ->
                    onCreatedRef()
                    onDocumentCreatedRef(createdEntityId)
                  }
              }
            },
          ),
        ),
    )
  }
}
