package co.typie.screen.space.folder

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.form.FormState
import co.typie.graphql.type.EntityVisibility
import co.typie.icons.Lucide
import co.typie.overlay.LocalToast
import co.typie.overlay.ToastType
import co.typie.platform.PlatformModule
import co.typie.platform.rememberFilePicker
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Img
import co.typie.ui.component.SelectField
import co.typie.ui.component.SelectFieldItem
import co.typie.ui.component.Text
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

private const val THUMBNAIL_WIDTH_DP = 64
private const val THUMBNAIL_HEIGHT_DP = 38

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
fun FolderShareContent(
  model: FolderViewModel,
  folderId: String,
  folderUrl: String,
  initialVisibility: EntityVisibility,
  initialThumbnailUrl: String?,
  onUpdated: () -> Unit = {},
) {
  val share = PlatformModule.share
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  val scope = rememberCoroutineScope()
  val form =
    remember(folderId, initialVisibility, initialThumbnailUrl) {
      FolderShareForm(scope, initialVisibility, initialThumbnailUrl)
    }
  var isUpdatingVisibility by remember { mutableStateOf(false) }
  var isUploadingThumbnail by remember { mutableStateOf(false) }
  var isRemovingThumbnail by remember { mutableStateOf(false) }
  var isSharing by remember { mutableStateOf(false) }
  var recursiveApplyPhase by remember(folderId) { mutableStateOf(RecursiveApplyPhase.Idle) }
  var recursiveApplyResetJob by remember(folderId) { mutableStateOf<Job?>(null) }
  val isApplyingRecursive = recursiveApplyPhase == RecursiveApplyPhase.Inflight
  val isBusy =
    isUpdatingVisibility ||
      isUploadingThumbnail ||
      isRemovingThumbnail ||
      isApplyingRecursive ||
      isSharing

  fun updateVisibility(nextVisibility: EntityVisibility) {
    if (isUpdatingVisibility) return

    if (form.visibility.initialValue == nextVisibility) return

    isUpdatingVisibility = true
    scope.launch {
      model
        .updateFolderVisibility(folderId = folderId, visibility = nextVisibility)
        .withDefaultExceptionHandler(toast)
        .onOk {
          form.visibility.commit()
          onUpdated()
        }
        .onException { form.visibility.rollback() }
      isUpdatingVisibility = false
    }
  }

  fun removeThumbnail() {
    if (isUploadingThumbnail || isRemovingThumbnail) return

    form.thumbnailUrl.setValue(null)
    isRemovingThumbnail = true
    scope.launch {
      model
        .removeFolderThumbnail(folderId = folderId)
        .withDefaultExceptionHandler(toast)
        .onOk {
          form.thumbnailUrl.commit()
          onUpdated()
        }
        .onException { form.thumbnailUrl.rollback() }
      isRemovingThumbnail = false
    }
  }

  fun applyRecursiveVisibility() {
    if (isApplyingRecursive || isUpdatingVisibility) return

    recursiveApplyResetJob?.cancel()
    recursiveApplyPhase = RecursiveApplyPhase.Inflight
    scope.launch {
      model
        .applyFolderVisibilityRecursively(folderId = folderId, visibility = form.visibility.value)
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

    if (folderUrl.isBlank()) {
      toast.show(ToastType.Error, "폴더 링크를 공유할 수 없어요.")
      return
    }

    isSharing = true
    try {
      // TODO: Track folder share action.
      val shared = share.share(folderUrl)
      if (!shared) {
        toast.show(ToastType.Error, "폴더 링크를 공유할 수 없어요.")
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
        .uploadFolderThumbnail(folderId = folderId, file = file)
        .withDefaultExceptionHandler(toast)
        .onOk { thumbnailResult ->
          form.thumbnailUrl.setValue(thumbnailResult.url)
          form.thumbnailUrl.commit()
          onUpdated()
        }
      isUploadingThumbnail = false
    }
  }

  SheetLayout(
    header = {
      ActionHeader(
        title = "이 폴더 공유하기",
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
      FolderShareSection(title = "폴더 조회 권한") {
        FolderShareOptionRow(
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
              enabled = !isUpdatingVisibility && !isApplyingRecursive,
              onSelected = { nextVisibility -> updateVisibility(nextVisibility) },
            )
          },
        )
      }

      FolderShareSection(title = "썸네일") {
        FolderShareOptionRow(
          icon = Lucide.Image,
          label = "미리보기 이미지",
          trailing = {
            FolderThumbnailControl(
              thumbnailUrl = form.thumbnailUrl.value,
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
                      message = "현재 폴더의 미리보기 이미지를 삭제합니다.",
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

@Composable
private fun FolderShareSection(title: String, content: @Composable ColumnScope.() -> Unit) {
  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(16.dp),
    content = {
      Text(text = title, style = AppTheme.typography.caption, color = AppTheme.colors.textSecondary)
      content()
    },
  )
}

@Composable
private fun FolderShareOptionRow(icon: IconData, label: String, trailing: @Composable () -> Unit) {
  Row(
    modifier = Modifier.fillMaxWidth().heightIn(min = 24.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(icon = icon, modifier = Modifier.size(20.dp), tint = AppTheme.colors.textSecondary)

    Spacer(Modifier.size(8.dp))

    Text(
      text = label,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.body,
      color = AppTheme.colors.textSecondary,
    )

    trailing()
  }
}

@Composable
private fun FolderThumbnailControl(
  thumbnailUrl: String?,
  isUploading: Boolean,
  isRemoving: Boolean,
  onUploadClick: () -> Unit,
  onRemoveClick: () -> Unit,
) {
  Row(
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    FolderThumbnailUploadButton(
      thumbnailUrl = thumbnailUrl,
      isUploading = isUploading,
      enabled = !isRemoving,
      onClick = onUploadClick,
    )

    if (thumbnailUrl != null) {
      FolderThumbnailRemoveButton(
        enabled = !isUploading && !isRemoving,
        isRemoving = isRemoving,
        onClick = onRemoveClick,
      )
    }
  }
}

@Composable
private fun FolderThumbnailUploadButton(
  thumbnailUrl: String?,
  isUploading: Boolean,
  enabled: Boolean,
  onClick: () -> Unit,
) {
  val shape = RoundedCornerShape(6.dp)

  InteractionScope {
    Box(
      modifier =
        Modifier.then(if (enabled) Modifier.clickable(onClick = onClick) else Modifier)
          .then(if (enabled) Modifier.pressScale(0.95f) else Modifier),
      contentAlignment = Alignment.Center,
    ) {
      Box(
        modifier =
          Modifier.size(width = THUMBNAIL_WIDTH_DP.dp, height = THUMBNAIL_HEIGHT_DP.dp)
            .clip(shape)
            .background(AppTheme.colors.surfaceSunken, shape)
            .border(
              width = 1.dp,
              color =
                if (thumbnailUrl == null) AppTheme.colors.borderStrong
                else AppTheme.colors.borderSubtle,
              shape = shape,
            ),
        contentAlignment = Alignment.Center,
      ) {
        when {
          thumbnailUrl != null -> {
            Img(url = thumbnailUrl, modifier = Modifier.fillMaxSize().clip(shape))
          }

          isUploading -> {
            FolderThumbnailSpinner()
          }

          else -> {
            Icon(
              icon = Lucide.Image,
              modifier = Modifier.size(14.dp),
              tint = AppTheme.colors.textTertiary,
            )
          }
        }
      }
    }
  }
}

@Composable
private fun FolderThumbnailRemoveButton(
  enabled: Boolean,
  isRemoving: Boolean,
  onClick: () -> Unit,
) {
  InteractionScope {
    Box(
      modifier =
        Modifier.heightIn(min = THUMBNAIL_HEIGHT_DP.dp)
          .clickable(enabled = enabled) { onClick() }
          .pressScale(0.95f),
      contentAlignment = Alignment.Center,
    ) {
      Text(
        text = if (isRemoving) "삭제 중..." else "삭제",
        modifier = Modifier.padding(horizontal = 8.dp),
        style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W600),
        color = if (enabled && !isRemoving) AppTheme.colors.danger else AppTheme.colors.textTertiary,
      )
    }
  }
}

@Composable
private fun FolderThumbnailSpinner() {
  val transition = rememberInfiniteTransition()
  val spinnerColor = AppTheme.colors.textTertiary
  val rotation by
    transition.animateFloat(
      initialValue = 0f,
      targetValue = 360f,
      animationSpec = infiniteRepeatable(animation = tween(1000, easing = LinearEasing)),
    )

  Canvas(Modifier.size(14.dp)) {
    drawArc(
      color = spinnerColor,
      startAngle = rotation,
      sweepAngle = 220f,
      useCenter = false,
      style = Stroke(width = 1.5.dp.toPx(), cap = StrokeCap.Round),
    )
  }
}
