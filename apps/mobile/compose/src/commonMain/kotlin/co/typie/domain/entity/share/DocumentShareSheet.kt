package co.typie.domain.entity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.domain.settings.SettingSwitch
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.form.FormState
import co.typie.graphql.fragment.DocumentShare_entity
import co.typie.graphql.type.DocumentContentRating
import co.typie.graphql.type.EntityVisibility
import co.typie.icons.Lucide
import co.typie.platform.FilePickerResult
import co.typie.platform.PickedFile
import co.typie.platform.PlatformModule
import co.typie.platform.rememberFilePicker
import co.typie.result.Result
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.SelectField
import co.typie.ui.component.SelectFieldItem
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.theme.AppTheme
import kotlin.random.Random
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

private class DocumentShareForm(
  scope: CoroutineScope,
  initialVisibility: EntityVisibility,
  initialContentRating: DocumentContentRating,
  initialHasPassword: Boolean,
  initialPassword: String,
  initialThumbnailUrl: String?,
  initialAllowReaction: Boolean,
  initialProtectContent: Boolean,
) : FormState(scope) {
  val visibility = field(initialVisibility) { focusable = false }
  val contentRating = field(initialContentRating) { focusable = false }
  val hasPassword = field(initialHasPassword) { focusable = false }
  val password = field(initialPassword) { focusable = false }
  val thumbnailUrl = field(initialThumbnailUrl) { focusable = false }
  val allowReaction = field(initialAllowReaction) { focusable = false }
  val protectContent = field(initialProtectContent) { focusable = false }
}

private data class DocumentContentRatingOption(
  val rating: DocumentContentRating,
  val label: String,
  val icon: IconData? = null,
)

private data class DocumentReactionOption(
  val allowReaction: Boolean,
  val label: String,
  val icon: IconData,
)

internal interface DocumentShareSheetModel {
  suspend fun updateDocumentsVisibility(
    documentIds: List<String>,
    visibility: EntityVisibility,
  ): Result<Unit, Nothing>

  suspend fun updateDocumentsContentRating(
    documentIds: List<String>,
    contentRating: DocumentContentRating,
  ): Result<Unit, Nothing>

  suspend fun updateDocumentsPassword(
    documentIds: List<String>,
    password: String?,
  ): Result<Unit, Nothing>

  suspend fun updateDocumentsAllowReaction(
    documentIds: List<String>,
    allowReaction: Boolean,
  ): Result<Unit, Nothing>

  suspend fun updateDocumentsProtectContent(
    documentIds: List<String>,
    protectContent: Boolean,
  ): Result<Unit, Nothing>

  suspend fun uploadDocumentsThumbnail(
    documentIds: List<String>,
    file: PickedFile,
  ): Result<ShareThumbnailResult, Nothing>

  suspend fun removeDocumentsThumbnail(documentIds: List<String>): Result<Unit, Nothing>
}

private fun documentContentRatingOptions(): List<DocumentContentRatingOption> {
  return listOf(
    DocumentContentRatingOption(rating = DocumentContentRating.ALL, label = "없음"),
    DocumentContentRatingOption(rating = DocumentContentRating.R15, label = "15세"),
    DocumentContentRatingOption(rating = DocumentContentRating.R19, label = "성인"),
  )
}

private fun documentReactionOptions(): List<DocumentReactionOption> {
  return listOf(
    DocumentReactionOption(allowReaction = true, label = "누구나", icon = Lucide.UsersRound),
    DocumentReactionOption(allowReaction = false, label = "비허용", icon = Lucide.Ban),
  )
}

private fun generateDocumentSharePassword(): String {
  return List(4) { Random.nextInt(10).toString() }.joinToString("")
}

private val DocumentShare_entity.document
  get() = requireNotNull(node.onDocument)

private fun resolveDocumentShareTitle(count: Int): String {
  return if (count <= 1) "이 문서 공유하기" else "문서 ${count}개 공유하기"
}

@Composable
context(_: SheetScope<Unit>)
internal fun DocumentShareSheet(
  model: DocumentShareSheetModel,
  documents: List<DocumentShare_entity>,
  loading: Boolean = false,
  onUpdated: () -> Unit = {},
) {
  val share = PlatformModule.share
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  val scope = rememberCoroutineScope()
  val documentIds = remember(documents) { documents.map { it.document.id } }
  val documentUrls = remember(documents) { documents.map(DocumentShare_entity::url) }
  val visibilityValues = remember(documents) { documents.map(DocumentShare_entity::visibility) }
  val contentRatingValues = remember(documents) { documents.map { it.document.contentRating } }
  val passwordValues = remember(documents) { documents.map { it.document.password } }
  val allowReactionValues = remember(documents) { documents.map { it.document.allowReaction } }
  val protectContentValues = remember(documents) { documents.map { it.document.protectContent } }
  val isSingleDocument = documents.size == 1
  val initialVisibility = documents.firstOrNull()?.visibility ?: EntityVisibility.PRIVATE
  val initialContentRating =
    documents.firstOrNull()?.document?.contentRating ?: DocumentContentRating.ALL
  val initialPassword = resolveSharedValue(passwordValues)
  val initiallyMixedVisibility = remember(documents) { hasMixedValues(visibilityValues) }
  val initiallyMixedContentRating = remember(documents) { hasMixedValues(contentRatingValues) }
  val initiallyMixedPasswordProtection =
    remember(documents) { hasMixedValues(documents.map { it.document.password != null }) }
  val initialThumbnailUrl = documents.firstOrNull()?.document?.thumbnail?.url
  val initiallyMixedThumbnail =
    remember(documents) { hasMixedValues(documents.map { it.document.thumbnail?.id }) }
  val initialAllowReaction = documents.firstOrNull()?.document?.allowReaction ?: true
  val initiallyMixedAllowReaction = remember(documents) { hasMixedValues(allowReactionValues) }
  val initialProtectContent = documents.firstOrNull()?.document?.protectContent ?: true
  val initiallyMixedProtectContent = remember(documents) { hasMixedValues(protectContentValues) }
  val form =
    remember(documentIds) {
      DocumentShareForm(
        scope = scope,
        initialVisibility = initialVisibility,
        initialContentRating = initialContentRating,
        initialHasPassword = initialPassword != null,
        initialPassword = initialPassword.orEmpty(),
        initialThumbnailUrl = if (initiallyMixedThumbnail) null else initialThumbnailUrl,
        initialAllowReaction = initialAllowReaction,
        initialProtectContent = initialProtectContent,
      )
    }
  var isUpdatingVisibility by remember { mutableStateOf(false) }
  var isUpdatingContentRating by remember { mutableStateOf(false) }
  var isUpdatingPassword by remember { mutableStateOf(false) }
  var isUploadingThumbnail by remember { mutableStateOf(false) }
  var isRemovingThumbnail by remember { mutableStateOf(false) }
  var isUpdatingAllowReaction by remember { mutableStateOf(false) }
  var isUpdatingProtectContent by remember { mutableStateOf(false) }
  var isSharing by remember { mutableStateOf(false) }
  var hasMixedVisibility by remember(documentIds) { mutableStateOf(initiallyMixedVisibility) }
  var committedHasMixedVisibility by
    remember(documentIds) { mutableStateOf(initiallyMixedVisibility) }
  var hasMixedContentRating by remember(documentIds) { mutableStateOf(initiallyMixedContentRating) }
  var committedHasMixedContentRating by
    remember(documentIds) { mutableStateOf(initiallyMixedContentRating) }
  var hasMixedPasswordProtection by
    remember(documentIds) { mutableStateOf(initiallyMixedPasswordProtection) }
  var committedHasMixedPasswordProtection by
    remember(documentIds) { mutableStateOf(initiallyMixedPasswordProtection) }
  var hasMixedThumbnail by remember(documentIds) { mutableStateOf(initiallyMixedThumbnail) }
  var committedHasMixedThumbnail by
    remember(documentIds) { mutableStateOf(initiallyMixedThumbnail) }
  var hasMixedAllowReaction by remember(documentIds) { mutableStateOf(initiallyMixedAllowReaction) }
  var committedHasMixedAllowReaction by
    remember(documentIds) { mutableStateOf(initiallyMixedAllowReaction) }
  var hasMixedProtectContent by
    remember(documentIds) { mutableStateOf(initiallyMixedProtectContent) }
  var committedHasMixedProtectContent by
    remember(documentIds) { mutableStateOf(initiallyMixedProtectContent) }
  val isBusy =
    isUpdatingVisibility ||
      isUpdatingContentRating ||
      isUpdatingPassword ||
      isUploadingThumbnail ||
      isRemovingThumbnail ||
      isUpdatingAllowReaction ||
      isUpdatingProtectContent ||
      isSharing

  fun updateVisibility(nextVisibility: EntityVisibility) {
    if (loading) return
    if (isUpdatingVisibility) return
    if (!hasMixedVisibility && form.visibility.initialValue == nextVisibility) return

    isUpdatingVisibility = true
    scope.launch {
      model
        .updateDocumentsVisibility(documentIds = documentIds, visibility = nextVisibility)
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

  fun updateContentRating(nextContentRating: DocumentContentRating) {
    if (loading) return
    if (isUpdatingContentRating) return
    if (!hasMixedContentRating && form.contentRating.initialValue == nextContentRating) return

    isUpdatingContentRating = true
    scope.launch {
      model
        .updateDocumentsContentRating(documentIds = documentIds, contentRating = nextContentRating)
        .withDefaultExceptionHandler(toast)
        .onOk {
          hasMixedContentRating = false
          committedHasMixedContentRating = false
          form.contentRating.commit()
          onUpdated()
        }
        .onException {
          hasMixedContentRating = committedHasMixedContentRating
          form.contentRating.rollback()
        }
      isUpdatingContentRating = false
    }
  }

  fun updateAllowReaction(nextAllowReaction: Boolean) {
    if (loading) return
    if (isUpdatingAllowReaction) return
    if (!hasMixedAllowReaction && form.allowReaction.initialValue == nextAllowReaction) return

    isUpdatingAllowReaction = true
    scope.launch {
      model
        .updateDocumentsAllowReaction(documentIds = documentIds, allowReaction = nextAllowReaction)
        .withDefaultExceptionHandler(toast)
        .onOk {
          hasMixedAllowReaction = false
          committedHasMixedAllowReaction = false
          form.allowReaction.commit()
          onUpdated()
        }
        .onException {
          hasMixedAllowReaction = committedHasMixedAllowReaction
          form.allowReaction.rollback()
        }
      isUpdatingAllowReaction = false
    }
  }

  fun updateProtectContent(nextProtectContent: Boolean) {
    if (loading) return
    if (isUpdatingProtectContent) return

    form.protectContent.setValue(nextProtectContent)
    hasMixedProtectContent = false
    isUpdatingProtectContent = true
    scope.launch {
      model
        .updateDocumentsProtectContent(
          documentIds = documentIds,
          protectContent = nextProtectContent,
        )
        .withDefaultExceptionHandler(toast)
        .onOk {
          hasMixedProtectContent = false
          committedHasMixedProtectContent = false
          form.protectContent.commit()
          onUpdated()
        }
        .onException {
          hasMixedProtectContent = committedHasMixedProtectContent
          form.protectContent.rollback()
        }
      isUpdatingProtectContent = false
    }
  }

  fun commitPassword(password: String) {
    if (loading) return
    if (isUpdatingPassword) return

    val nextPassword = password.trim()
    if (nextPassword.isEmpty()) {
      toast.show(ToastType.Notification, "비밀번호를 입력해주세요.")
      return
    }

    if (
      !hasMixedPasswordProtection &&
        form.hasPassword.initialValue &&
        form.password.initialValue.trim() == nextPassword
    ) {
      return
    }

    form.hasPassword.setValue(true)
    form.password.setValue(nextPassword)
    hasMixedPasswordProtection = false
    isUpdatingPassword = true
    scope.launch {
      model
        .updateDocumentsPassword(documentIds = documentIds, password = nextPassword)
        .withDefaultExceptionHandler(toast)
        .onOk {
          hasMixedPasswordProtection = false
          committedHasMixedPasswordProtection = false
          form.hasPassword.commit()
          form.password.commit()
          onUpdated()
        }
        .onException {
          hasMixedPasswordProtection = committedHasMixedPasswordProtection
          form.hasPassword.rollback()
          form.password.rollback()
        }
      isUpdatingPassword = false
    }
  }

  fun updatePasswordProtection(nextEnabled: Boolean) {
    if (loading) return
    if (isUpdatingPassword) return

    form.hasPassword.setValue(nextEnabled)
    hasMixedPasswordProtection = false

    if (nextEnabled) {
      val currentPassword = form.password.value.trim()
      if (currentPassword.isNotEmpty()) {
        commitPassword(currentPassword)
      }
      return
    }

    form.password.setValue("")
    isUpdatingPassword = true
    scope.launch {
      model
        .updateDocumentsPassword(documentIds = documentIds, password = null)
        .withDefaultExceptionHandler(toast)
        .onOk {
          hasMixedPasswordProtection = false
          committedHasMixedPasswordProtection = false
          form.hasPassword.commit()
          form.password.commit()
          onUpdated()
        }
        .onException {
          hasMixedPasswordProtection = committedHasMixedPasswordProtection
          form.hasPassword.rollback()
          form.password.rollback()
        }
      isUpdatingPassword = false
    }
  }

  fun removeThumbnail() {
    if (loading) return
    if (isUploadingThumbnail || isRemovingThumbnail) return

    form.thumbnailUrl.setValue(null)
    isRemovingThumbnail = true
    scope.launch {
      model
        .removeDocumentsThumbnail(documentIds = documentIds)
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

  suspend fun shareDocument() {
    if (loading) return
    if (isSharing) return

    val shareText = resolveEntityShareText(documentUrls)
    if (shareText == null) {
      toast.show(ToastType.Error, "문서 링크를 공유할 수 없어요.")
      return
    }

    isSharing = true
    try {
      if (!share.share(shareText)) {
        toast.show(ToastType.Error, "문서 링크를 공유할 수 없어요.")
      }
    } finally {
      isSharing = false
    }
  }

  val filePicker = rememberFilePicker { result ->
    if (loading) {
      (result as? FilePickerResult.Selected)?.files?.forEach { it.close() }
      return@rememberFilePicker
    }
    val file =
      when (result) {
        FilePickerResult.Cancelled -> return@rememberFilePicker
        is FilePickerResult.Failed -> {
          toast.error("대표 이미지를 불러오지 못했어요.")
          return@rememberFilePicker
        }
        is FilePickerResult.Selected -> result.files.first()
      }
    if (isUploadingThumbnail || isRemovingThumbnail) {
      file.close()
      return@rememberFilePicker
    }

    isUploadingThumbnail = true
    val uploadJob = scope.launch {
      try {
        model
          .uploadDocumentsThumbnail(documentIds = documentIds, file = file)
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
      } finally {
        isUploadingThumbnail = false
      }
    }
    uploadJob.invokeOnCompletion { file.close() }
  }

  SheetLayout(
    header = {
      SheetBar(
        leading = {
          SheetBarTextButton(
            text = "완료",
            color = AppTheme.colors.textDefault,
            enabled = !isBusy,
            onClick = { dismiss() },
          )
        },
        center = {
          Text(
            text = resolveDocumentShareTitle(documents.size),
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        },
      )
    }
  ) {
    Column(verticalArrangement = Arrangement.spacedBy(32.dp)) {
      ShareSection(title = "문서 조회 권한") {
        ShareOptionRow(
          icon = Lucide.Blend,
          label = "공개 범위",
          trailing = {
            Skeleton(enabled = loading) {
              SelectField(
                field = form.visibility,
                items =
                  listOf(
                    SelectFieldItem(
                      value = EntityVisibility.PUBLIC,
                      label = "공개",
                      description = "누구나 볼 수 있고 스페이스에 노출돼요.",
                      icon = Lucide.Globe,
                    ),
                    SelectFieldItem(
                      value = EntityVisibility.UNLISTED,
                      label = "링크가 있는 사람",
                      description = "링크가 있는 누구나 볼 수 있어요.",
                      icon = Lucide.Link,
                    ),
                    SelectFieldItem(
                      value = EntityVisibility.PRIVATE,
                      label = "비공개",
                      description = "나만 볼 수 있어요.",
                      icon = Lucide.Lock,
                    ),
                  ),
                values = visibilityValues,
                enabled = !loading && !isUpdatingVisibility,
                onSelected = ::updateVisibility,
              )
            }
          },
        )

        ShareOptionRow(
          icon = Lucide.IdCard,
          label = "연령 제한",
          trailing = {
            Skeleton(enabled = loading) {
              SelectField(
                field = form.contentRating,
                items =
                  documentContentRatingOptions().map { option ->
                    SelectFieldItem(value = option.rating, label = option.label, icon = option.icon)
                  },
                values = contentRatingValues,
                enabled = !loading && !isUpdatingContentRating,
                onSelected = ::updateContentRating,
              )
            }
          },
        )

        ShareOptionRow(
          icon = Lucide.LockKeyhole,
          label = "비밀번호 보호",
          trailing = {
            Skeleton(enabled = loading) {
              SettingSwitch(
                checked = form.hasPassword.value,
                indeterminate = hasMixedPasswordProtection,
                enabled = !loading && !isUpdatingPassword,
                onCheckedChange = ::updatePasswordProtection,
              )
            }
          },
        )

        if (!loading && form.hasPassword.value) {
          TextField(
            value = form.password.value,
            onValueChange = { form.password.setValue(it) },
            label = "비밀번호",
            placeholder = "비밀번호를 입력해주세요.",
            keyboardType = KeyboardType.Number,
            imeAction = ImeAction.Done,
            onImeAction = { commitPassword(form.password.value) },
            onBlur = { commitPassword(form.password.value) },
            suffix = {
              InteractionScope {
                Box(
                  modifier =
                    Modifier.size(28.dp)
                      .clickable(enabled = !loading && !isUpdatingPassword) {
                        commitPassword(generateDocumentSharePassword())
                      }
                      .pressScale(0.95f),
                  contentAlignment = Alignment.Center,
                ) {
                  Icon(
                    icon = Lucide.Dices,
                    modifier = Modifier.size(18.dp),
                    tint = AppTheme.colors.textMuted,
                  )
                }
              }
            },
          )
        }
      }

      ShareSection(title = "썸네일") {
        ShareOptionRow(
          icon = Lucide.Image,
          label = "미리보기 이미지",
          trailing = {
            Skeleton(enabled = loading) {
              ShareThumbnailControl(
                thumbnailUrl = form.thumbnailUrl.value,
                isMixed = hasMixedThumbnail,
                isUploading = isUploadingThumbnail,
                isRemoving = isRemovingThumbnail,
                onUploadClick = {
                  if (!loading && !isUploadingThumbnail && !isRemovingThumbnail) {
                    filePicker("image/*")
                  }
                },
                onRemoveClick = {
                  scope.launch {
                    val result =
                      dialog.confirm(
                        title = "썸네일을 삭제할까요?",
                        message =
                          if (isSingleDocument) "현재 문서의 미리보기 이미지를 삭제합니다."
                          else "선택한 문서들의 미리보기 이미지를 삭제합니다.",
                        confirmText = "삭제",
                        confirmIsDestructive = true,
                      )
                    if (result is DialogResult.Resolved) {
                      removeThumbnail()
                    }
                  }
                },
              )
            }
          },
        )
      }

      ShareSection(title = "문서 상호작용") {
        ShareOptionRow(
          icon = Lucide.Smile,
          label = "이모지 반응",
          trailing = {
            Skeleton(enabled = loading) {
              SelectField(
                field = form.allowReaction,
                items =
                  documentReactionOptions().map { option ->
                    SelectFieldItem(
                      value = option.allowReaction,
                      label = option.label,
                      icon = option.icon,
                    )
                  },
                values = allowReactionValues,
                enabled = !loading && !isUpdatingAllowReaction,
                onSelected = ::updateAllowReaction,
              )
            }
          },
        )
      }

      ShareSection(title = "문서 보호") {
        ShareOptionRow(
          icon = Lucide.Shield,
          label = "내용 보호",
          trailing = {
            Skeleton(enabled = loading) {
              SettingSwitch(
                checked = form.protectContent.value,
                indeterminate = hasMixedProtectContent,
                enabled = !loading && !isUpdatingProtectContent,
                onCheckedChange = ::updateProtectContent,
              )
            }
          },
        )
      }

      Button(
        text = "공유하기",
        enabled = !loading && !isSharing,
        loading = isSharing,
        onClick = { shareDocument() },
      )
    }
  }
}
