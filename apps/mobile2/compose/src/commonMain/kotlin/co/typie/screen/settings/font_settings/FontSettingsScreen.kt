package co.typie.screen.settings.font_settings

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.overlay.ToastType
import co.typie.platform.FilePickerSelectionMode
import co.typie.platform.rememberFilePicker
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.screen.subscription.planUpgradeRoute
import co.typie.screen.subscription.showPlanUpgradeSheet
import co.typie.service.CurrentSubscriptionStore
import co.typie.service.hasSubscriptionOrNull
import co.typie.ui.component.AlertModal
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.ConfirmModal
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.FontSpecimen
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.bottomsheet.BottomSheetScaffold
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.component.familySpecimenFallbacks
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.component.weightSpecimenFallbacks
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.time.Duration
import kotlinx.coroutines.launch

private data class PendingFontDeletion(val familyDisplayName: String, val font: FontSettingsFont)

@Composable
fun FontSettingsScreen() {
  val model = viewModel { FontSettingsViewModel() }
  val nav = Nav.current
  val toast = LocalToast.current
  val currentSubscriptionStore = CurrentSubscriptionStore
  val scrollState = rememberScrollState()
  val scope = rememberCoroutineScope()
  val bottomSheetHost = LocalBottomSheetHost.current
  var pendingFamilyDeletion by remember { mutableStateOf<FontSettingsFamily?>(null) }
  var pendingFontDeletion by remember { mutableStateOf<PendingFontDeletion?>(null) }
  val currentSubscriptionState by currentSubscriptionStore.state.collectAsState()

  val filePicker =
    rememberFilePicker(selectionMode = FilePickerSelectionMode.Multiple) { files ->
      if (files.isEmpty()) return@rememberFilePicker
      scope.launch {
        model
          .uploadFonts(files)
          .collect(
            onPending = { progress ->
              toast.show(
                ToastType.Loading,
                "폰트 업로드 중... (${progress.current}/${progress.total})",
                Duration.ZERO,
              )
            },
            onSettled = { result ->
              toast.dismiss()
              result.withDefaultExceptionHandler(toast).onOk { summary ->
                model.state.uploadSummary = summary
              }
            },
          )
      }
    }

  fun requestUpload() {
    if (model.query.state !is QueryState.Success) return
    if (model.state.isUploading) return

    when (currentSubscriptionState.hasSubscriptionOrNull()?.let(::fontUploadAction)) {
      FontUploadAction.PickFont -> {
        scope.launch {
          bottomSheetHost.show {
            FontUploadSheet(
              isUploading = model.state.isUploading,
              uploadCurrentIndex = model.state.uploadCurrentIndex,
              uploadTotalCount = model.state.uploadTotalCount,
              onUploadClick = {
                dismiss()
                filePicker("*/*")
              },
            )
          }
        }
      }

      FontUploadAction.ShowPlanUpgradeSheet -> {
        scope.launch {
          planUpgradeRoute(
              bottomSheetHost.showPlanUpgradeSheet(
                message = "폰트 업로드 기능은 FULL ACCESS 플랜에서 사용할 수 있어요."
              )
            )
            ?.let { route -> nav.navigate(route) }
        }
      }

      null -> return
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("폰트", style = AppTheme.typography.title) },
    trailing = { TopBarButton(Lucide.Plus, onClick = { requestUpload() }) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    scrollState = scrollState,
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    Text("폰트", style = AppTheme.typography.display, modifier = Modifier.padding(top = 4.dp))

    SectionTitle("직접 업로드한 폰트")

    if (model.userFontFamilies.isEmpty()) {
      FontSettingsEmptyState()
    } else {
      model.userFontFamilies.forEach { family ->
        FontSettingsFamilySection(
          family = family,
          deletingFamilyId = model.state.deletingFamilyId,
          deletingFontId = model.state.deletingFontId,
          onDeleteFamilyClick = { pendingFamilyDeletion = family },
          onDeleteFontClick = { font ->
            pendingFontDeletion =
              PendingFontDeletion(familyDisplayName = family.displayName, font = font)
          },
        )
      }
    }

    Spacer(Modifier.height(72.dp))
  }

  pendingFamilyDeletion?.let { family ->
    ConfirmModal(
      title = "폰트 패밀리 삭제",
      message = "\"${family.displayName}\" 폰트 패밀리 전체를 삭제하시겠어요?",
      confirmText = "삭제",
      confirmIsDestructive = true,
      onConfirm = {
        pendingFamilyDeletion = null
        model.deleteFamily(family).withDefaultExceptionHandler(toast).onOk {
          toast.show(ToastType.Success, "\"${family.displayName}\" 폰트 패밀리를 삭제했어요.")
        }
      },
      onDismiss = { pendingFamilyDeletion = null },
    )
  }

  pendingFontDeletion?.let { pending ->
    ConfirmModal(
      title = "폰트 삭제",
      message =
        "\"${pending.familyDisplayName} ${fontWeightLabel(pending.font.weight, pending.font.subfamilyDisplayName)}\" 폰트를 삭제하시겠어요?",
      confirmText = "삭제",
      confirmIsDestructive = true,
      onConfirm = {
        pendingFontDeletion = null
        model.deleteFont(pending.font).withDefaultExceptionHandler(toast).onOk {
          toast.show(
            ToastType.Success,
            "\"${pending.familyDisplayName} ${fontWeightLabel(pending.font.weight, pending.font.subfamilyDisplayName)}\" 폰트를 삭제했어요.",
          )
        }
      },
      onDismiss = { pendingFontDeletion = null },
    )
  }

  model.state.uploadSummary?.let { summary ->
    val (title, message) = fontUploadSummaryDisplay(summary)
    AlertModal(
      title = title,
      message = message,
      onConfirm = { model.dismissUploadSummary() },
      onDismiss = { model.dismissUploadSummary() },
    )
  }
}

@Composable
private fun BottomSheetScope<Unit>.FontUploadSheet(
  isUploading: Boolean,
  uploadCurrentIndex: Int,
  uploadTotalCount: Int,
  onUploadClick: suspend () -> Unit,
) {
  val loadingText =
    if (isUploading && uploadTotalCount > 0) {
      "업로드 중... (${uploadCurrentIndex.coerceAtLeast(1)}/$uploadTotalCount)"
    } else {
      "업로드 중..."
    }

  BottomSheetScaffold(title = "폰트 업로드하기") {
    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        Text("이용 안내", style = AppTheme.typography.label)

        FontSettingsBullet("TTF 확장자를 가진 폰트 파일만 업로드할 수 있어요.")
        FontSettingsBullet("여러 개의 TTF 폰트 파일을 한 번에 선택할 수 있어요.")
        FontSettingsBullet("기울어진 폰트는 업로드할 수 없어요.")
        FontSettingsBullet("업로드한 폰트는 내 글이라면 어디서나 사용할 수 있어요.")
        FontSettingsBullet("무료 폰트이거나 웹 사용 라이선스가 있는 폰트만 이용해 주세요.")
        FontSettingsBullet("저작권에 위배되는 폰트는 삭제될 수 있어요.")
      }
    }

    Button(
      text = "폰트 파일 선택",
      loading = isUploading,
      loadingText = loadingText,
      onClick = onUploadClick,
    )
  }
}

@Composable
private fun FontSettingsBullet(text: String) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(8.dp),
    verticalAlignment = Alignment.Top,
  ) {
    Text("•", style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
    Text(
      text,
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
      modifier = Modifier.weight(1f),
    )
  }
}

@Composable
private fun FontSettingsEmptyState() {
  CardSurface(modifier = Modifier.fillMaxWidth()) {
    Text(
      "아직 직접 업로드한 폰트가 없어요.\n우측 상단의 추가 버튼으로 TTF 폰트를 한 번에 여러 개 업로드할 수 있어요.",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
      modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 40.dp),
    )
  }
}

@Composable
private fun FontSettingsFamilySection(
  family: FontSettingsFamily,
  deletingFamilyId: String?,
  deletingFontId: String?,
  onDeleteFamilyClick: suspend () -> Unit,
  onDeleteFontClick: suspend (FontSettingsFont) -> Unit,
) {
  val representativeFont = representativeFont(family.fonts)
  val isDeleteActionEnabled = deletingFamilyId == null && deletingFontId == null
  val familySpecimenFallback =
    familySpecimenFallbacks(displayName = family.displayName, familyName = family.familyName)

  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    FontSpecimen(
      text = family.displayName,
      fontId = representativeFont?.id,
      weight = representativeFont?.weight,
      style = AppTheme.typography.title,
      modifier = Modifier.fillMaxWidth(),
      fallbackTexts = familySpecimenFallback,
    )

    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column {
        family.fonts.forEachIndexed { index, font ->
          val fontLabel = fontWeightLabel(font.weight, font.subfamilyDisplayName)

          Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            FontSpecimen(
              text = fontLabel,
              fontId = font.id,
              weight = font.weight,
              style = AppTheme.typography.label,
              modifier = Modifier.weight(1f),
              fallbackTexts =
                weightSpecimenFallbacks(
                  label = fontLabel,
                  subfamilyDisplayName = font.subfamilyDisplayName,
                  weight = font.weight,
                ),
            )

            FontDeleteButton(
              enabled = isDeleteActionEnabled,
              isDeleting = deletingFontId == font.id,
              onClick = { onDeleteFontClick(font) },
            )
          }

          if (index < family.fonts.lastIndex) {
            CardDivider()
          }
        }

        if (family.fonts.size > 1) {
          CardDivider()
          FontDeleteFamilyRow(
            enabled = isDeleteActionEnabled,
            isDeleting = deletingFamilyId == family.id,
            onClick = onDeleteFamilyClick,
          )
        }
      }
    }
  }
}

@Composable
private fun FontDeleteButton(enabled: Boolean, isDeleting: Boolean, onClick: suspend () -> Unit) {
  if (isDeleting) {
    Text(
      text = "삭제 중...",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
    )
    return
  }
  InteractionScope {
    Box(
      modifier = Modifier.size(44.dp).then(if (enabled) Modifier.clickable(onClick) else Modifier),
      contentAlignment = Alignment.Center,
    ) {
      Icon(
        icon = Lucide.Trash2,
        modifier = Modifier.size(18.dp).pressScale(0.92f),
        tint = if (enabled) AppTheme.colors.textSecondary else AppTheme.colors.textTertiary,
      )
    }
  }
}

private fun fontUploadErrorMessage(error: FontUploadError): String {
  return when (error) {
    FontUploadError.UnsupportedFormat -> "TTF 파일만 업로드할 수 있어요."
    FontUploadError.InvalidFontStyle -> "기울어진 폰트는 업로드할 수 없어요."
    FontUploadError.UploadFailed -> "폰트 업로드에 실패했어요."
    FontUploadError.RefreshFailed -> "폰트 목록을 새로고침하지 못했어요. 화면을 다시 열어주세요."
  }
}

private fun fontUploadSummaryDisplay(summary: FontUploadSummary): Pair<String, String> {
  val title =
    when (summary.status) {
      FontUploadSummaryStatus.Success -> "폰트 업로드 완료"
      FontUploadSummaryStatus.PartialSuccess -> "폰트 업로드 일부 완료"
      FontUploadSummaryStatus.Failure -> "폰트 업로드 실패"
    }

  val sections = buildList {
    if (summary.successes.isNotEmpty()) {
      val successesByFamily = linkedMapOf<String, MutableList<FontUploadSuccess>>()
      summary.successes.forEach { success ->
        successesByFamily.getOrPut(success.familyId) { mutableListOf() }.add(success)
      }

      val successLines =
        successesByFamily.values.map { familyUploads ->
          val familyDisplayName = familyUploads.first().familyDisplayName
          val labels =
            familyUploads
              .sortedBy { it.weight }
              .map { fontWeightLabel(it.weight, it.subfamilyDisplayName) }
              .joinToString(", ")
          "\u2022 $familyDisplayName ($labels)"
        }

      add("${summary.successes.size}개의 폰트가 추가되었어요.\n\n${successLines.joinToString("\n")}")
    }

    if (summary.failures.isNotEmpty()) {
      val failureLines =
        summary.failures.joinToString("\n") { failure ->
          val errorMessage = fontUploadErrorMessage(failure.error)
          if (failure.name.isNotEmpty()) "\u2022 ${failure.name}: $errorMessage"
          else "\u2022 $errorMessage"
        }
      add("${summary.failures.size}개의 폰트 업로드에 실패했어요.\n\n$failureLines")
    }
  }

  val note =
    if (summary.status == FontUploadSummaryStatus.Success) {
      "업로드한 폰트는 이 화면에서 관리할 수 있어요."
    } else {
      null
    }

  val message = buildString {
    append(sections.joinToString("\n\n"))
    if (note != null) {
      append("\n\n")
      append(note)
    }
  }

  return title to message
}

@Composable
private fun FontDeleteFamilyRow(
  enabled: Boolean,
  isDeleting: Boolean,
  onClick: suspend () -> Unit,
) {
  if (isDeleting) {
    Box(
      modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 14.dp),
      contentAlignment = Alignment.Center,
    ) {
      Text(
        text = "삭제 중...",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
      )
    }
    return
  }

  InteractionScope {
    Row(
      modifier =
        Modifier.fillMaxWidth()
          .then(if (enabled) Modifier.clickable(onClick) else Modifier)
          .padding(horizontal = 16.dp, vertical = 14.dp)
          .pressScale(),
      horizontalArrangement = Arrangement.Center,
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Text(
        "이 폰트 패밀리 전체 삭제",
        style = AppTheme.typography.action,
        color = if (enabled) AppTheme.colors.danger else AppTheme.colors.textTertiary,
      )
    }
  }
}
