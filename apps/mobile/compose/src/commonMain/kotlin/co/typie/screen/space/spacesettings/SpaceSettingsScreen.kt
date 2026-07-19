package co.typie.screen.space.spacesettings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
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
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.subscription.Entitlement
import co.typie.domain.subscription.GatedAction
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.gate
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.navigationBarsOrImePadding
import co.typie.ext.pressScale
import co.typie.ext.safeDrawingHorizontalPadding
import co.typie.ext.thenIf
import co.typie.ext.verticalScroll
import co.typie.graphql.fragment.Img_image
import co.typie.graphql.type.SiteDateDisplay
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.FilePickerResult
import co.typie.platform.rememberFilePicker
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.AlertBanner
import co.typie.ui.component.AlertBannerVariant
import co.typie.ui.component.BottomFade
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Img
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.SelectField
import co.typie.ui.component.SelectFieldItem
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.alert
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.time.Duration
import kotlinx.coroutines.launch

private val SpaceDateDisplayOptions =
  mapOf(
    SiteDateDisplay.CREATED_AT to "최초 생성 시각",
    SiteDateDisplay.UPDATED_AT to "마지막 수정 시각",
    SiteDateDisplay.NONE to "미표시",
  )

@Composable
fun SpaceSettingsScreen() {
  val model = viewModel { SpaceSettingsViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current
  val sheet = LocalSheet.current

  val filePicker = rememberFilePicker { result ->
    val file =
      when (result) {
        FilePickerResult.Cancelled -> return@rememberFilePicker
        is FilePickerResult.Failed -> {
          toast.error("로고 이미지를 불러오지 못했어요.")
          return@rememberFilePicker
        }
        is FilePickerResult.Selected -> result.files.first()
      }
    val uploadJob = scope.launch {
      model
        .uploadLogo(file)
        .collect(
          onPending = { toast.show(ToastType.Loading, "로고 업로드 중...", Duration.ZERO) },
          onSettled = { result ->
            result.withDefaultExceptionHandler(toast).onOk {
              toast.show(ToastType.Success, "로고가 업로드되었어요.")
            }
          },
        )
    }
    uploadJob.invokeOnCompletion { file.close() }
  }

  ProvideTopBar(
    center = { Text("스페이스 설정", style = AppTheme.typography.title) },
    trailing = { MoreMenu(model) },
  )

  Screen(
    loadable = model.query,
    contentPadding = PaddingValues.Zero,
    overlay = {
      BottomFade(modifier = Modifier.padding(horizontal = 16.dp)) {
        ToastAnchor()

        Button(
          text = "저장",
          loading = model.isSubmitting,
          loadingText = "저장 중...",
          onClick = {
            if (!SubscriptionService.gate(sheet, nav, GatedAction.UpdateSpace)) return@Button
            model.submit().withDefaultExceptionHandler(toast).onOk {
              toast.success("스페이스 설정이 변경되었어요.")
              nav.pop()
            }
          },
        )

        Spacer(Modifier.height(12.dp))
      }
    },
  ) { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .navigationBarsOrImePadding()
          .padding(AppTheme.spacings.scrollBottomPadding)
    ) {
      SpaceLogoHero(
        image = model.query.data.site.logo.img_image,
        previewUrl = model.logoPreviewUrl,
        onClick = {
          scope.launch {
            if (SubscriptionService.gate(sheet, nav, GatedAction.UpdateSpace)) {
              filePicker("image/*")
            }
          }
        },
        topInset = contentPadding.calculateTopPadding(),
      )

      PaperPane(modifier = Modifier.offset(y = (-28).dp)) {
        Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
          Text(text = "일반", style = AppTheme.typography.title)

          TextField(
            field = model.form.name,
            label = "이름",
            labelPosition = LabelPosition.Internal,
            placeholder = "스페이스 이름",
          )

          Box(
            modifier =
              Modifier.fillMaxWidth().thenIf(
                SubscriptionService.entitlement is Entitlement.Expired
              ) {
                clickable { SubscriptionService.gate(sheet, nav, GatedAction.ChangeSpaceAddress) }
              }
          ) {
            TextField(
              field = model.form.slug,
              label = "주소",
              help =
                if (SubscriptionService.entitlement is Entitlement.Expired) {
                  "스페이스 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요."
                } else null,
              labelPosition = LabelPosition.Internal,
              placeholder = "스페이스 주소",
              enabled = SubscriptionService.entitlement is Entitlement.Active,
              readOnly = SubscriptionService.entitlement !is Entitlement.Active,
              suffix = {
                Text(
                  ".${model.usersiteHost}",
                  style = AppTheme.typography.body,
                  color = AppTheme.colors.textMuted,
                )
              },
            )

            if (SubscriptionService.entitlement is Entitlement.Expired) {
              LockedBadge(
                modifier = Modifier.align(Alignment.TopEnd).padding(top = 8.dp, end = 12.dp)
              )
            }
          }
        }

        Column {
          Text(text = "디자인", style = AppTheme.typography.title)

          Spacer(Modifier.height(12.dp))

          SpaceSettingsRow(
            label = "글 목록에 표시할 날짜",
            trailing = {
              SelectField(
                field = model.form.dateDisplay,
                items =
                  SpaceDateDisplayOptions.map { option ->
                    SelectFieldItem(value = option.key, label = option.value)
                  },
              )
            },
          )
        }
      }
    }
  }
}

@Composable
private fun SpaceSettingsRow(label: String, trailing: @Composable RowScope.() -> Unit) {
  Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
    Text(
      text = label,
      style = AppTheme.typography.label,
      modifier = Modifier.weight(1f),
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )

    trailing()
  }
}

@Composable
private fun MoreMenu(model: SpaceSettingsViewModel) {
  val scope = rememberCoroutineScope()

  val nav = Nav.current
  val toast = LocalToast.current
  val sheet = LocalSheet.current
  val dialog = LocalDialog.current
  val colors = AppTheme.colors

  PopoverMenu(anchor = { TopBarButton(icon = Lucide.Ellipsis) }) {
    item(icon = Lucide.Trash2, label = "스페이스 삭제", color = colors.danger) {
      if (model.query.data.me.sites.size <= 1) {
        scope.launch {
          dialog.alert(
            title = "스페이스를 삭제할 수 없어요",
            message = "계정에는 최소 1개의 스페이스가 필요해요.\n새 스페이스를 만든 후 삭제할 수 있어요.",
          )
        }
      } else {
        scope.launch {
          sheet.present {
            DeleteSiteSheet(
              documentCount = model.query.data.site.documentCount,
              folderCount = model.query.data.site.folderCount,
              isDeleting = model.isDeleting,
              onDelete = {
                model.deleteSite().withDefaultExceptionHandler(toast).onOk {
                  toast.success("스페이스가 삭제되었어요.")
                  complete(Unit)
                  nav.pop()
                }
              },
            )
          }
        }
      }
    }
  }
}

@Composable
private fun SpaceLogo(image: Img_image, previewUrl: String?, onClick: () -> Unit) {
  val logoShape = AppShapes.squircle(AppShapes.lg)

  InteractionScope {
    Box(modifier = Modifier.clickable(onClick).pressScale()) {
      Box(
        modifier =
          Modifier.size(104.dp)
            .clip(logoShape)
            .border(1.dp, AppTheme.colors.borderDefault, logoShape)
            .background(AppTheme.colors.surfaceDefault, logoShape),
        contentAlignment = Alignment.Center,
      ) {
        if (previewUrl != null) {
          Img(url = previewUrl, modifier = Modifier.fillMaxSize())
        } else {
          Img(image = image, modifier = Modifier.fillMaxSize())
        }
      }

      Box(
        modifier =
          Modifier.align(Alignment.BottomEnd)
            .offset(x = 6.dp, y = 6.dp)
            .size(36.dp)
            .background(AppTheme.colors.surfaceDefault, AppShapes.circle)
            .border(1.dp, AppTheme.colors.borderDefault, AppShapes.circle),
        contentAlignment = Alignment.Center,
      ) {
        Icon(
          icon = Lucide.Camera,
          modifier = Modifier.size(18.dp),
          tint = AppTheme.colors.textMuted,
        )
      }
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
private fun DeleteSiteSheet(
  documentCount: Int,
  folderCount: Int,
  isDeleting: Boolean,
  onDelete: suspend context(SheetScope<Unit>) () -> Unit,
) {
  var inputValue by remember { mutableStateOf("") }

  val confirmText = "$documentCount"
  val isConfirmed = documentCount == 0 || inputValue == confirmText

  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = "스페이스 삭제",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    },
    footer = {
      Button(
        text = "삭제",
        variant = ButtonVariant.Danger,
        enabled = isConfirmed,
        loading = isDeleting,
        loadingText = "삭제 중...",
        onClick = {
          if (!isConfirmed || isDeleting) return@Button
          onDelete()
        },
      )
    },
  ) {
    AlertBanner(
      text =
        when {
          folderCount > 0 && documentCount > 0 ->
            "${folderCount}개의 폴더와 ${documentCount}개의 문서가 함께 삭제돼요."
          folderCount > 0 -> "${folderCount}개의 폴더가 함께 삭제돼요."
          documentCount > 0 -> "${documentCount}개의 문서가 함께 삭제돼요."
          else -> "스페이스가 비어있어요."
        },
      variant =
        if (folderCount == 0 && documentCount == 0) AlertBannerVariant.Default
        else AlertBannerVariant.Danger,
    )

    if (documentCount > 0) {
      TextField(
        value = inputValue,
        onValueChange = { inputValue = it },
        label = "확인 숫자",
        help = "삭제를 진행하려면 스페이스와 함께 삭제되는 문서 수($documentCount)를 입력해주세요.",
        helpTextStyle = AppTheme.typography.caption,
        placeholder = confirmText,
        autoFocus = true,
        keyboardType = KeyboardType.Number,
      )
    }
  }
}

@Composable
private fun SpaceLogoHero(
  image: Img_image,
  previewUrl: String?,
  onClick: () -> Unit,
  topInset: Dp,
) {
  Box(modifier = Modifier.fillMaxWidth().height(200.dp + topInset)) {
    Box(modifier = Modifier.matchParentSize().blur(64.dp), contentAlignment = Alignment.Center) {
      if (previewUrl != null) {
        Img(url = previewUrl, modifier = Modifier.matchParentSize().scale(1.3f))
      } else {
        Img(image = image, modifier = Modifier.matchParentSize().scale(1.3f))
      }
    }

    Box(modifier = Modifier.matchParentSize().background(Color.Black.copy(alpha = 0.2f)))

    Column(
      modifier = Modifier.matchParentSize().padding(top = topInset + 8.dp),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      SpaceLogo(image = image, previewUrl = previewUrl, onClick = onClick)
    }
  }
}

@Composable
private fun PaperPane(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
  val shape = RoundedCornerShape(topStart = AppShapes.xl, topEnd = AppShapes.xl)
  Column(
    modifier =
      modifier
        .fillMaxSize()
        .background(AppTheme.colors.surfaceCanvas, shape)
        .safeDrawingHorizontalPadding()
        .padding(horizontal = 16.dp, vertical = 24.dp),
    verticalArrangement = Arrangement.spacedBy(24.dp),
    content = content,
  )
}

@Composable
private fun LockedBadge(modifier: Modifier = Modifier) {
  Row(
    modifier =
      modifier
        .background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.sm))
        .border(1.dp, AppTheme.colors.borderDefault, AppShapes.rounded(AppShapes.sm))
        .padding(horizontal = 6.dp, vertical = 3.dp),
    horizontalArrangement = Arrangement.spacedBy(4.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(icon = Lucide.Lock, modifier = Modifier.size(10.dp), tint = AppTheme.colors.textMuted)
    Text(text = "잠김", style = AppTheme.typography.micro, color = AppTheme.colors.textHint)
  }
}
