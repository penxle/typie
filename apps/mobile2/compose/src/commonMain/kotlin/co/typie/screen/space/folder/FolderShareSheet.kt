package co.typie.screen.space.folder

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import co.typie.form.FormState
import co.typie.graphql.type.EntityVisibility
import co.typie.icons.Lucide
import co.typie.overlay.LocalToast
import co.typie.platform.PlatformFile
import co.typie.platform.PlatformModule
import co.typie.platform.rememberFilePicker
import co.typie.result.Result
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.screen.space.entity.EntityShareKind
import co.typie.screen.space.entity.ShareOptionRow
import co.typie.screen.space.entity.ShareSection
import co.typie.screen.space.entity.ShareThumbnailControl
import co.typie.screen.space.entity.ShareThumbnailResult
import co.typie.screen.space.entity.hasMixedValues
import co.typie.screen.space.entity.resolveEntityShareText
import co.typie.screen.space.entity.resolveEntityShareTitle
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.SelectField
import co.typie.ui.component.SelectFieldItem
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.sheet.ActionHeader
import co.typie.ui.component.sheet.HeaderTextAction
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private enum class RecursiveApplyPhase {
  Idle,
  Inflight,
  Success,
}

private class FolderShareForm(
  scope: CoroutineScope,
  initialVisibility: EntityVisibility,
  initialThumbnailUrl: String?,
) : FormState(scope) {
  val visibility = field(initialVisibility) { focusable = false }
  val thumbnailUrl = field(initialThumbnailUrl) { focusable = false }
}

private data class FolderVisibilityOption(
  val visibility: EntityVisibility,
  val label: String,
  val description: String,
  val icon: IconData,
)

internal interface FolderShareSheetModel {
  suspend fun updateFoldersVisibility(
    folderIds: List<String>,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing>

  suspend fun uploadFoldersThumbnail(
    folderIds: List<String>,
    file: PlatformFile,
  ): Result<ShareThumbnailResult, Nothing>

  suspend fun removeFoldersThumbnail(folderIds: List<String>): Result<Unit, Nothing>

  suspend fun applyFoldersVisibilityRecursively(
    folderIds: List<String>,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing>
}

internal data class FolderShareTarget(
  val id: String,
  val url: String,
  val visibility: EntityVisibility,
  val thumbnailId: String?,
  val thumbnailUrl: String?,
)

private fun folderVisibilityOptions(): List<FolderVisibilityOption> {
  return listOf(
    FolderVisibilityOption(
      visibility = EntityVisibility.PUBLIC,
      label = "공개",
      description = "누구나 볼 수 있고 스페이스에 노출돼요.",
      icon = Lucide.Globe,
    ),
    FolderVisibilityOption(
      visibility = EntityVisibility.UNLISTED,
      label = "링크가 있는 사람",
      description = "링크가 있는 누구나 볼 수 있어요.",
      icon = Lucide.Link,
    ),
    FolderVisibilityOption(
      visibility = EntityVisibility.PRIVATE,
      label = "비공개",
      description = "나만 볼 수 있어요.",
      icon = Lucide.Lock,
    ),
  )
}

@Composable
context(_: SheetScope<Unit>)
internal fun FolderShareContent(
  model: FolderShareSheetModel,
  folders: List<FolderShareTarget>,
  onUpdated: () -> Unit = {},
) {
  val share = PlatformModule.share
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  val scope = rememberCoroutineScope()
  val folderIds = remember(folders) { folders.map(FolderShareTarget::id) }
  val folderUrls = remember(folders) { folders.map(FolderShareTarget::url) }
  val visibilityValues = remember(folders) { folders.map(FolderShareTarget::visibility) }
  val isSingleFolder = folders.size == 1
  val initialVisibility = folders.firstOrNull()?.visibility ?: EntityVisibility.PRIVATE
  val initiallyMixedVisibility = remember(folders) { hasMixedValues(visibilityValues) }
  val initialThumbnailUrl = folders.firstOrNull()?.thumbnailUrl
  val initiallyMixedThumbnail =
    remember(folders) { hasMixedValues(folders.map(FolderShareTarget::thumbnailId)) }
  val form =
    remember(folderIds) {
      FolderShareForm(
        scope,
        initialVisibility,
        if (initiallyMixedThumbnail) null else initialThumbnailUrl,
      )
    }
  var isUpdatingVisibility by remember { mutableStateOf(false) }
  var isUploadingThumbnail by remember { mutableStateOf(false) }
  var isRemovingThumbnail by remember { mutableStateOf(false) }
  var isSharing by remember { mutableStateOf(false) }
  var hasMixedVisibility by remember(folderIds) { mutableStateOf(initiallyMixedVisibility) }
  var committedHasMixedVisibility by
    remember(folderIds) { mutableStateOf(initiallyMixedVisibility) }
  var hasMixedThumbnail by remember(folderIds) { mutableStateOf(initiallyMixedThumbnail) }
  var committedHasMixedThumbnail by remember(folderIds) { mutableStateOf(initiallyMixedThumbnail) }
  var recursiveApplyPhase by remember(folderIds) { mutableStateOf(RecursiveApplyPhase.Idle) }
  var recursiveApplyResetJob by remember(folderIds) { mutableStateOf<Job?>(null) }
  val isApplyingRecursive = recursiveApplyPhase == RecursiveApplyPhase.Inflight
  val isBusy =
    isUpdatingVisibility ||
      isUploadingThumbnail ||
      isRemovingThumbnail ||
      isApplyingRecursive ||
      isSharing

  fun updateVisibility(nextVisibility: EntityVisibility) {
    if (isUpdatingVisibility) return
    if (!hasMixedVisibility && form.visibility.initialValue == nextVisibility) return

    isUpdatingVisibility = true
    scope.launch {
      model
        .updateFoldersVisibility(folderIds = folderIds, visibility = nextVisibility)
        .withDefaultExceptionHandler(toast)
        .onOk {
          hasMixedVisibility = false
          committedHasMixedVisibility = false
          form.visibility.commit()
          onUpdated()
        }
        .onException {
          hasMixedVisibility = committedHasMixedVisibility
          form.visibility.rollback()
        }
      isUpdatingVisibility = false
    }
  }

  fun removeThumbnail() {
    if (isUploadingThumbnail || isRemovingThumbnail) return

    form.thumbnailUrl.setValue(null)
    isRemovingThumbnail = true
    scope.launch {
      model
        .removeFoldersThumbnail(folderIds = folderIds)
        .withDefaultExceptionHandler(toast)
        .onOk {
          hasMixedThumbnail = false
          committedHasMixedThumbnail = false
          form.thumbnailUrl.commit()
          onUpdated()
        }
        .onException {
          hasMixedThumbnail = committedHasMixedThumbnail
          form.thumbnailUrl.rollback()
        }
      isRemovingThumbnail = false
    }
  }

  fun applyRecursiveVisibility() {
    if (isApplyingRecursive || isUpdatingVisibility) return

    recursiveApplyResetJob?.cancel()
    recursiveApplyPhase = RecursiveApplyPhase.Inflight
    scope.launch {
      model
        .applyFoldersVisibilityRecursively(
          folderIds = folderIds,
          visibility = form.visibility.value,
        )
        .withDefaultExceptionHandler(toast)
        .onOk {
          onUpdated()
          recursiveApplyResetJob?.cancel()
          recursiveApplyPhase = RecursiveApplyPhase.Success
          recursiveApplyResetJob = scope.launch {
            delay(2_000)
            recursiveApplyPhase = RecursiveApplyPhase.Idle
          }
        }
        .onException {
          recursiveApplyResetJob?.cancel()
          recursiveApplyPhase = RecursiveApplyPhase.Idle
        }
    }
  }

  suspend fun shareFolder() {
    if (isSharing) return

    val shareText = resolveEntityShareText(folderUrls)
    if (shareText == null) {
      toast.show(co.typie.overlay.ToastType.Error, "폴더 링크를 공유할 수 없어요.")
      return
    }

    isSharing = true
    try {
      if (!share.share(shareText)) {
        toast.show(co.typie.overlay.ToastType.Error, "폴더 링크를 공유할 수 없어요.")
      }
    } finally {
      isSharing = false
    }
  }

  val filePicker = rememberFilePicker { files ->
    val file = files.firstOrNull() ?: return@rememberFilePicker
    if (isUploadingThumbnail || isRemovingThumbnail) return@rememberFilePicker

    isUploadingThumbnail = true
    scope.launch {
      model
        .uploadFoldersThumbnail(folderIds = folderIds, file = file)
        .withDefaultExceptionHandler(toast)
        .onOk { thumbnailResult ->
          hasMixedThumbnail = false
          committedHasMixedThumbnail = false
          form.thumbnailUrl.setValue(thumbnailResult.url)
          form.thumbnailUrl.commit()
          onUpdated()
        }
        .onException {
          hasMixedThumbnail = committedHasMixedThumbnail
          form.thumbnailUrl.rollback()
        }
      isUploadingThumbnail = false
    }
  }

  SheetLayout(
    header = {
      ActionHeader(
        title = resolveEntityShareTitle(EntityShareKind.Folder, folders.size),
        leading = {
          HeaderTextAction(
            text = "완료",
            color = AppTheme.colors.brand,
            textStyle = AppTheme.typography.action.copy(fontWeight = FontWeight.W700),
            enabled = !isBusy,
            onClick = { dismiss() },
          )
        },
      )
    }
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(32.dp)) {
      ShareSection(title = "폴더 조회 권한") {
        ShareOptionRow(
          icon = Lucide.Blend,
          label = "공개 범위",
          trailing = {
            SelectField(
              field = form.visibility,
              items =
                folderVisibilityOptions().map { option ->
                  SelectFieldItem(
                    value = option.visibility,
                    label = option.label,
                    description = option.description,
                    icon = option.icon,
                  )
                },
              values = visibilityValues,
              enabled = !isUpdatingVisibility && !isApplyingRecursive,
              onSelected = ::updateVisibility,
            )
          },
        )
      }

      ShareSection(title = "썸네일") {
        ShareOptionRow(
          icon = Lucide.Image,
          label = "미리보기 이미지",
          trailing = {
            ShareThumbnailControl(
              thumbnailUrl = form.thumbnailUrl.value,
              isMixed = hasMixedThumbnail,
              isUploading = isUploadingThumbnail,
              isRemoving = isRemovingThumbnail,
              onUploadClick = {
                if (!isUploadingThumbnail && !isRemovingThumbnail) {
                  filePicker("image/*")
                }
              },
              onRemoveClick = {
                scope.launch {
                  val result =
                    dialog.confirm(
                      title = "썸네일을 삭제할까요?",
                      message =
                        if (isSingleFolder) "현재 폴더의 미리보기 이미지를 삭제합니다."
                        else "선택한 폴더들의 미리보기 이미지를 삭제합니다.",
                      confirmText = "삭제",
                      confirmIsDestructive = true,
                    )
                  if (result is DialogResult.Resolved) {
                    removeThumbnail()
                  }
                }
              },
            )
          },
        )
      }

      Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        when (recursiveApplyPhase) {
          RecursiveApplyPhase.Inflight ->
            Button(
              text = "하위 항목에 동일한 설정 적용하기",
              variant = ButtonVariant.Secondary,
              enabled = !isUpdatingVisibility && !isApplyingRecursive,
              loading = true,
              loadingText = "적용 중...",
              onClick = { applyRecursiveVisibility() },
            )

          RecursiveApplyPhase.Success ->
            Button(
              text = "적용됨",
              leading = { color ->
                Icon(icon = Lucide.Check, modifier = Modifier.size(16.dp), tint = color)
              },
              variant = ButtonVariant.Secondary,
              enabled = !isUpdatingVisibility && !isApplyingRecursive,
              onClick = { applyRecursiveVisibility() },
            )

          RecursiveApplyPhase.Idle ->
            Button(
              text = "하위 항목에 동일한 설정 적용하기",
              leading = { color ->
                Icon(icon = Lucide.Layers2, modifier = Modifier.size(16.dp), tint = color)
              },
              variant = ButtonVariant.Secondary,
              enabled = !isUpdatingVisibility && !isApplyingRecursive,
              onClick = { applyRecursiveVisibility() },
            )
        }

        Button(
          text = "공유하기",
          enabled = !isSharing,
          loading = isSharing,
          onClick = { shareFolder() },
        )
      }
    }
  }
}
