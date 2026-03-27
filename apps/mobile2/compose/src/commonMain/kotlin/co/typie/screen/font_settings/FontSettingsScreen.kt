package co.typie.screen.font_settings

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.platform.rememberFilePicker
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.ConfirmModal
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import coil3.compose.AsyncImagePainter
import coil3.compose.rememberAsyncImagePainter
import kotlinx.coroutines.launch
import org.koin.compose.viewmodel.koinViewModel

private data class PendingFontDeletion(
  val familyDisplayName: String,
  val font: FontSettingsFont,
)

@Composable
fun FontSettingsScreen() {
  val model = koinViewModel<FontSettingsViewModel>()
  val scrollState = rememberScrollState()
  val scope = rememberCoroutineScope()
  val bottomSheetHost = LocalBottomSheetHost.current
  var pendingFamilyDeletion by remember { mutableStateOf<FontSettingsFamily?>(null) }
  var pendingFontDeletion by remember { mutableStateOf<PendingFontDeletion?>(null) }

  val filePicker = rememberFilePicker { file ->
    if (file == null) return@rememberFilePicker
    scope.launch { model.uploadFont(file) }
  }

  fun requestUpload() {
    if (model.query.state !is QueryState.Success) return
    if (model.state.isUploading) return
    scope.launch {
      bottomSheetHost.show {
        FontUploadSheet(
          hasSubscription = model.hasSubscription,
          isUploading = model.state.isUploading,
          onUploadClick = {
            when (fontUploadAction(model.hasSubscription)) {
              FontUploadAction.PickFont -> {
                dismiss()
                filePicker("*/*")
              }

              FontUploadAction.ShowSubscriptionNotice -> {
                model.showUploadSubscriptionNotice()
              }
            }
          },
        )
      }
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
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
  ) { contentPadding ->
    Column(
      modifier = Modifier
        .fillMaxSize()
        .verticalScroll(scrollState)
        .padding(contentPadding)
        .navigationBarsPadding(),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text(
        "폰트",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

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
              pendingFontDeletion = PendingFontDeletion(
                familyDisplayName = family.displayName,
                font = font,
              )
            },
          )
        }
      }

      Spacer(Modifier.height(72.dp))
    }
  }

  pendingFamilyDeletion?.let { family ->
    ConfirmModal(
      title = "폰트 패밀리 삭제",
      message = "\"${family.displayName}\" 폰트 패밀리 전체를 삭제하시겠어요?",
      confirmText = "삭제",
      confirmIsDestructive = true,
      onConfirm = {
        model.deleteFamily(family)
        pendingFamilyDeletion = null
      },
      onDismiss = { pendingFamilyDeletion = null },
    )
  }

  pendingFontDeletion?.let { pending ->
    ConfirmModal(
      title = "폰트 삭제",
      message = "\"${pending.familyDisplayName} ${fontWeightLabel(pending.font.weight, pending.font.subfamilyDisplayName)}\" 폰트를 삭제하시겠어요?",
      confirmText = "삭제",
      confirmIsDestructive = true,
      onConfirm = {
        model.deleteFont(
          familyDisplayName = pending.familyDisplayName,
          font = pending.font,
        )
        pendingFontDeletion = null
      },
      onDismiss = { pendingFontDeletion = null },
    )
  }
}

@Composable
private fun BottomSheetScope<Unit>.FontUploadSheet(
  hasSubscription: Boolean,
  isUploading: Boolean,
  onUploadClick: suspend () -> Unit,
) {
  Column(
    modifier = Modifier
      .fillMaxWidth()
      .padding(horizontal = 16.dp),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    Text(
      "폰트 업로드하기",
      style = AppTheme.typography.title,
    )

    CardSurface(
      modifier = Modifier.fillMaxWidth(),
    ) {
      Column(
        modifier = Modifier
          .fillMaxWidth()
          .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        Text(
          "이용 안내",
          style = AppTheme.typography.label,
        )

        FontSettingsBullet("TTF 확장자를 가진 폰트 파일만 업로드할 수 있어요.")
        FontSettingsBullet("기울어진 폰트는 업로드할 수 없어요.")
        FontSettingsBullet("업로드한 폰트는 내 글이라면 어디서나 사용할 수 있어요.")
        FontSettingsBullet("무료 폰트이거나 웹 사용 라이선스가 있는 폰트만 이용해 주세요.")
        FontSettingsBullet("저작권에 위배되는 폰트는 삭제될 수 있어요.")

        if (!hasSubscription) {
          Text(
            "폰트 업로드는 FULL ACCESS 플랜에서 사용할 수 있어요.",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textTertiary,
            modifier = Modifier.padding(top = 4.dp),
          )
        }
      }
    }

    Button(
      text = "폰트 파일 선택",
      loading = isUploading,
      loadingText = "업로드 중...",
      onClick = onUploadClick,
    )
  }
}

@Composable
private fun FontSettingsBullet(
  text: String,
) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(8.dp),
    verticalAlignment = Alignment.Top,
  ) {
    Text(
      "•",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
    )
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
  CardSurface(
    modifier = Modifier.fillMaxWidth(),
  ) {
    Text(
      "아직 직접 업로드한 폰트가 없어요.\n우측 상단의 추가 버튼으로 TTF 폰트를 업로드할 수 있어요.",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
      modifier = Modifier
        .fillMaxWidth()
        .padding(horizontal = 20.dp, vertical = 40.dp),
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

  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    FontSpecimen(
      text = family.displayName,
      fontId = representativeFont?.id,
      weight = representativeFont?.weight,
      style = AppTheme.typography.title,
      modifier = Modifier.fillMaxWidth(),
    )

    CardSurface(
      modifier = Modifier.fillMaxWidth(),
    ) {
      Column {
        family.fonts.forEachIndexed { index, font ->
          Row(
            modifier = Modifier
              .fillMaxWidth()
              .padding(horizontal = 16.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            FontSpecimen(
              text = fontWeightLabel(font.weight, font.subfamilyDisplayName),
              fontId = font.id,
              weight = font.weight,
              style = AppTheme.typography.label,
              modifier = Modifier.weight(1f),
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
private fun FontDeleteButton(
  enabled: Boolean,
  isDeleting: Boolean,
  onClick: suspend () -> Unit,
) {
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
      modifier = Modifier
        .size(44.dp)
        .then(if (enabled) Modifier.clickable(onClick) else Modifier),
      contentAlignment = Alignment.Center,
    ) {
      Icon(
        icon = Lucide.Trash2,
        modifier = Modifier
          .size(18.dp)
          .pressScale(0.92f),
        tint = if (enabled) AppTheme.colors.textSecondary else AppTheme.colors.textTertiary,
      )
    }
  }
}

@Composable
private fun FontDeleteFamilyRow(
  enabled: Boolean,
  isDeleting: Boolean,
  onClick: suspend () -> Unit,
) {
  if (isDeleting) {
    Box(
      modifier = Modifier
        .fillMaxWidth()
        .padding(horizontal = 16.dp, vertical = 14.dp),
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
      modifier = Modifier
        .fillMaxWidth()
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

@Composable
private fun FontSpecimen(
  text: String,
  fontId: String?,
  weight: Int?,
  style: androidx.compose.ui.text.TextStyle,
  modifier: Modifier = Modifier,
) {
  val fallbackStyle = if (weight != null) {
    style.copy(fontWeight = FontWeight(weight.coerceIn(1, 1000)))
  } else {
    style
  }
  val fallback: @Composable () -> Unit = {
    Text(
      text = text,
      style = fallbackStyle,
      modifier = Modifier.wrapContentWidth(),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }

  if (fontId == null) {
    Box(
      modifier = modifier,
      contentAlignment = Alignment.CenterStart,
    ) {
      fallback()
    }
    return
  }

  val specimenHeight = with(LocalDensity.current) { style.fontSize.toDp() } + 4.dp
  val specimenUrl = remember(fontId, text) { fontSpecimenUrl(fontId = fontId, text = text) }
  val painter = rememberAsyncImagePainter(model = specimenUrl)
  val painterState by painter.state.collectAsState()

  Box(
    modifier = modifier.heightIn(min = specimenHeight),
    contentAlignment = Alignment.CenterStart,
  ) {
    if (painterState is AsyncImagePainter.State.Success) {
      Image(
        painter = painter,
        contentDescription = null,
        contentScale = ContentScale.Fit,
        modifier = Modifier
          .height(specimenHeight)
          .wrapContentWidth(Alignment.Start),
      )
    } else {
      Box(
        modifier = Modifier
          .height(specimenHeight)
          .wrapContentWidth(Alignment.Start),
        contentAlignment = Alignment.CenterStart,
      ) {
        fallback()
      }
    }
  }
}
