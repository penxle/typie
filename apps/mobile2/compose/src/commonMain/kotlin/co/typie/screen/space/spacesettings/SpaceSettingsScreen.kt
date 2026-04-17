package co.typie.screen.space.spacesettings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.SubscriptionServiceState
import co.typie.domain.subscription.gate
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.excludeTop
import co.typie.ext.onlyTop
import co.typie.ext.pressScale
import co.typie.ext.thenIf
import co.typie.ext.verticalScroll
import co.typie.graphql.fragment.Img_image
import co.typie.graphql.type.SiteDateDisplay
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.rememberFilePicker
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.AlertBanner
import co.typie.ui.component.AlertBannerVariant
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Img
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.alert
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetOptionList
import co.typie.ui.component.sheet.SheetOptionRow
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

  val filePicker = rememberFilePicker { files ->
    val file = files.firstOrNull() ?: return@rememberFilePicker
    scope.launch {
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
  }

  ProvideTopBar(
    center = { Text("스페이스 설정", style = AppTheme.typography.title) },
    trailing = { MoreMenu(model) },
  )

  Screen(loadable = model.query) { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().padding(contentPadding.excludeTop())) {
      Box(modifier = Modifier.weight(1f)) {
        Column(
          modifier =
            Modifier.fillMaxSize()
              .verticalScroll(scrollState)
              .padding(contentPadding.onlyTop())
              .padding(AppTheme.spacings.scrollBottomPadding),
          verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          SectionTitle("일반")

          CardSurface(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.fillMaxWidth()) {
              Column(
                modifier = Modifier.fillMaxWidth().padding(top = 24.dp, bottom = 20.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(12.dp),
              ) {
                SpaceLogo(
                  image = model.query.data.site.logo.img_image,
                  previewUrl = model.logoPreviewUrl,
                  onClick = { filePicker("image/*") },
                )

                Text(
                  "스페이스 로고",
                  style = AppTheme.typography.caption,
                  color = AppTheme.colors.textTertiary,
                )
              }

              CardDivider()

              Column(
                modifier = Modifier.fillMaxWidth().padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
              ) {
                TextField(
                  field = model.form.name,
                  label = "이름",
                  labelPosition = LabelPosition.Internal,
                  placeholder = "스페이스 이름",
                )

                Box(
                  modifier =
                    Modifier.fillMaxWidth().thenIf(
                      SubscriptionService.state is SubscriptionServiceState.NotSubscribed
                    ) {
                      clickable {
                        SubscriptionService.gate(
                          sheet,
                          nav,
                          message = "스페이스 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.",
                        )
                      }
                    }
                ) {
                  TextField(
                    field = model.form.slug,
                    label = "주소",
                    help =
                      if (SubscriptionService.state is SubscriptionServiceState.NotSubscribed) {
                        "스페이스 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요."
                      } else {
                        null
                      },
                    helpTextStyle = AppTheme.typography.caption,
                    labelPosition = LabelPosition.Internal,
                    placeholder = "스페이스 주소",
                    enabled = SubscriptionService.state is SubscriptionServiceState.Subscribed,
                    readOnly = SubscriptionService.state !is SubscriptionServiceState.Subscribed,
                    suffix = {
                      Text(
                        ".${model.usersiteHost}",
                        style = AppTheme.typography.body,
                        color = AppTheme.colors.textSecondary,
                      )
                    },
                  )
                }
              }
            }
          }

          Column(
            modifier = Modifier.fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(12.dp),
          ) {
            SectionTitle("디자인")

            CardSurface(modifier = Modifier.fillMaxWidth()) {
              CardRow(
                onClick = {
                  val result = sheet.present {
                    SpaceDateDisplaySheet(selected = model.form.dateDisplay.value)
                  }

                  if (result != null) {
                    model.form.dateDisplay.setValue(result)
                  }
                }
              ) {
                SpaceSettingsRowContent(
                  label = "글 목록에 표시할 날짜",
                  trailing = {
                    Text(
                      text = SpaceDateDisplayOptions[model.form.dateDisplay.value] ?: "(알 수 없음)",
                      style = AppTheme.typography.caption,
                      color = AppTheme.colors.textTertiary,
                      maxLines = 1,
                      overflow = TextOverflow.Ellipsis,
                    )

                    Spacer(Modifier.width(4.dp))

                    Icon(
                      icon = Lucide.ChevronRight,
                      modifier = Modifier.size(16.dp),
                      tint = AppTheme.colors.textTertiary,
                    )
                  },
                )
              }
            }
          }
        }
      }

      ToastAnchor()

      Button(
        text = "저장",
        loading = model.isSubmitting,
        loadingText = "저장 중...",
        onClick = {
          model.submit().withDefaultExceptionHandler(toast).onOk {
            toast.success("스페이스 설정이 변경되었어요.")
            nav.pop()
          }
        },
      )
    }
  }
}

@Composable
context(rowScope: RowScope)
private fun SpaceSettingsRowContent(label: String, trailing: @Composable RowScope.() -> Unit) {
  Text(
    text = label,
    style = AppTheme.typography.label,
    modifier = with(rowScope) { Modifier.weight(1f) },
    maxLines = 1,
    overflow = TextOverflow.Ellipsis,
  )

  Row(
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(0.dp),
    content = trailing,
  )
}

@Composable
context(_: SheetScope<SiteDateDisplay>)
private fun SpaceDateDisplaySheet(selected: SiteDateDisplay) {
  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = "글 목록에 표시할 날짜",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textPrimary,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    }
  ) {
    SheetOptionList(items = SpaceDateDisplayOptions.entries) { (value, label) ->
      SheetOptionRow(selected = value == selected, onClick = { complete(value) }) {
        Text(
          text = label,
          style = AppTheme.typography.action,
          modifier = Modifier.fillMaxWidth(),
          color = AppTheme.colors.textPrimary,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }
    }
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
            .background(AppTheme.colors.surfaceRaised, AppShapes.circle)
            .border(1.dp, AppTheme.colors.borderDefault, AppShapes.circle),
        contentAlignment = Alignment.Center,
      ) {
        Icon(
          icon = Lucide.Camera,
          modifier = Modifier.size(18.dp),
          tint = AppTheme.colors.textSecondary,
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
  onDelete:
    suspend context(SheetScope<Unit>)
    () -> Unit,
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
            color = AppTheme.colors.textPrimary,
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
