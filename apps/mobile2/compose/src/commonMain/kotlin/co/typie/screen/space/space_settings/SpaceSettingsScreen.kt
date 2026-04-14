package co.typie.screen.space.space_settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
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
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.Img_image
import co.typie.graphql.type.SiteDateDisplay
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.overlay.ToastType
import co.typie.platform.rememberFilePicker
import co.typie.result.onErr
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.subscription.PlanUpgradeContent
import co.typie.subscription.PlanUpgradeSheetResult
import co.typie.subscription.SubscriptionCelebrationContent
import co.typie.subscription.SubscriptionService
import co.typie.subscription.SubscriptionServiceState
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
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.alert
import co.typie.ui.component.dialog.error
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.close
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetOptionList
import co.typie.ui.component.sheet.SheetOptionRow
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.time.Duration
import kotlinx.coroutines.launch

private data class SpaceDateDisplayOption(val value: SiteDateDisplay, val label: String)

private fun spaceDateDisplayOptions(): List<SpaceDateDisplayOption> {
  return listOf(
    SpaceDateDisplayOption(SiteDateDisplay.CREATED_AT, "최초 생성 시각"),
    SpaceDateDisplayOption(SiteDateDisplay.UPDATED_AT, "마지막 수정 시각"),
    SpaceDateDisplayOption(SiteDateDisplay.NONE, "미표시"),
  )
}

private fun spaceDateDisplayLabel(value: SiteDateDisplay): String {
  return spaceDateDisplayOptions().firstOrNull { it.value == value }?.label ?: "미표시"
}

private val SpaceDateDisplaySheetPadding =
  SheetPadding(header = PaddingValues(horizontal = 16.dp), body = PaddingValues(horizontal = 16.dp))

@Composable
fun SpaceSettingsScreen() {
  val nav = Nav.current
  val dialog = LocalDialog.current
  val model = viewModel { SpaceSettingsViewModel() }
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val sheet = LocalSheet.current
  val scrollState = rememberScrollState()
  val subscriptionState = SubscriptionService.state

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
    trailing = { MoreMenu(model, dialog = dialog) },
  )

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  Screen(
    scrollState = scrollState,
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(24.dp),
    imeAware = true,
    bottomBar = {
      Button(
        text = "저장",
        modifier = Modifier.padding(horizontal = 16.dp).padding(bottom = 16.dp),
        loading = model.isSubmitting,
        loadingText = "저장 중...",
        onClick = {
          scope.launch {
            model
              .submit()
              .withDefaultExceptionHandler(toast)
              .onOk { nav.pop() }
              .onErr { error ->
                when (error) {
                  SubmitError.SlugAlreadyExists -> toast.show(ToastType.Error, "이미 사용 중인 URL이에요.")
                  SubmitError.SubscriptionUnknown ->
                    toast.show(ToastType.Error, "이용권 상태를 확인하는 중이에요. 잠시 후 다시 시도해주세요.")
                }
              }
          }
        },
      )
    },
  ) {
    val data = model.query.data
    Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
      SectionTitle("일반")

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.fillMaxWidth()) {
          Column(
            modifier = Modifier.fillMaxWidth().padding(top = 24.dp, bottom = 20.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(12.dp),
          ) {
            SpaceLogo(
              image = data.site.logo.img_image,
              previewUrl = model.state.logoPreviewUrl,
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
              field = model.state.form.name,
              label = "이름",
              labelPosition = LabelPosition.Internal,
              placeholder = "스페이스 이름",
            )

            Box(
              modifier =
                Modifier.fillMaxWidth()
                  .then(
                    if (subscriptionState is SubscriptionServiceState.NotSubscribed) {
                      Modifier.clickable {
                        scope.launch {
                          val upgradeResult = sheet.present {
                            PlanUpgradeContent(message = "스페이스 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.")
                          }
                          if (upgradeResult is PlanUpgradeSheetResult.TrialStarted) {
                            sheet.present {
                              SubscriptionCelebrationContent(
                                title = "무료 체험이 시작됐어요!",
                                message = "2주간 타이피의 모든 기능을 자유롭게 이용해보세요.",
                              )
                            }
                          }
                          if (upgradeResult is PlanUpgradeSheetResult.Upgrade)
                            nav.navigate(Route.EnrollPlan)
                        }
                      }
                    } else {
                      Modifier
                    }
                  )
            ) {
              TextField(
                field = model.state.form.slug,
                label = "주소",
                help =
                  if (subscriptionState is SubscriptionServiceState.NotSubscribed) {
                    "스페이스 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요."
                  } else {
                    null
                  },
                helpTextStyle = AppTheme.typography.caption,
                labelPosition = LabelPosition.Internal,
                placeholder = "스페이스 주소",
                enabled = subscriptionState is SubscriptionServiceState.Subscribed,
                readOnly = subscriptionState !is SubscriptionServiceState.Subscribed,
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
              scope.launch {
                val result = sheet.present {
                  SpaceDateDisplayContent(selected = model.state.form.dateDisplay.value)
                }
                if (result != null) {
                  model.state.form.dateDisplay.setValue(result)
                }
              }
            }
          ) {
            SpaceSettingsRowContent(
              label = "글 목록에 표시할 날짜",
              trailing = {
                Text(
                  text = spaceDateDisplayLabel(model.state.form.dateDisplay.value),
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
private fun SpaceDateDisplayContent(selected: SiteDateDisplay) {
  SheetLayout(
    padding = SpaceDateDisplaySheetPadding,
    verticalSpacing = 8.dp,
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
    },
  ) {
    SheetOptionList(items = spaceDateDisplayOptions()) { item ->
      SheetOptionRow(selected = item.value == selected, onClick = { complete(item.value) }) {
        Text(
          text = item.label,
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
private fun MoreMenu(model: SpaceSettingsViewModel, dialog: Dialog) {
  val nav = Nav.current
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val sheet = LocalSheet.current

  Popover(
    placement = PopoverPlacement.BelowEnd,
    anchor = { TopBarButton(icon = Lucide.Ellipsis) },
    pane = {
      Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
        PopoverList(
          items =
            listOf(
              PopoverListItem(
                content = {
                  Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
                  ) {
                    Icon(
                      icon = Lucide.Trash2,
                      modifier = Modifier.size(18.dp),
                      tint = AppTheme.colors.danger,
                    )
                    Spacer(Modifier.width(12.dp))
                    Text(
                      "스페이스 삭제",
                      style = AppTheme.typography.action,
                      color = AppTheme.colors.danger,
                    )
                  }
                },
                onSelected = {
                  close()
                  val data = model.query.data
                  val isLastSite = data.me.sites.size <= 1
                  if (isLastSite) {
                    scope.launch {
                      dialog.alert(
                        title = "스페이스를 삭제할 수 없어요",
                        message = "최소 1개의 스페이스가 필요해요.\n새 스페이스를 만든 후 삭제할 수 있어요.",
                      )
                    }
                  } else {
                    scope.launch {
                      sheet.present {
                        DeleteSiteConfirmContent(
                          documentCount = data.site.documentCount,
                          folderCount = data.site.folderCount,
                          isDeleting = { model.isDeletingSite },
                          onDelete = {
                            if (!model.isDeletingSite) {
                              scope.launch {
                                model.deleteSite().withDefaultExceptionHandler(toast).onOk {
                                  toast.show(ToastType.Success, "스페이스가 삭제되었어요.")
                                  complete(Unit)
                                  nav.pop()
                                }
                              }
                            }
                          },
                        )
                      }
                    }
                  }
                },
              )
            )
        )
      }
    },
  )
}

@Composable
private fun SpaceLogo(image: Img_image, previewUrl: String?, onClick: () -> Unit) {
  val logoShape = RoundedCornerShape(14.dp)

  InteractionScope {
    Box(modifier = Modifier.clickable(onClick).pressScale()) {
      Box(
        modifier =
          Modifier.size(104.dp)
            .background(AppTheme.colors.surfaceDefault, logoShape)
            .border(1.dp, AppTheme.colors.borderDefault, logoShape),
        contentAlignment = Alignment.Center,
      ) {
        if (previewUrl != null) {
          Img(url = previewUrl, modifier = Modifier.size(104.dp).clip(logoShape))
        } else {
          Img(image = image, modifier = Modifier.size(104.dp).clip(logoShape))
        }
      }

      Box(
        modifier =
          Modifier.align(Alignment.BottomEnd)
            .offset(x = 6.dp, y = 6.dp)
            .size(36.dp)
            .clip(CircleShape)
            .background(AppTheme.colors.surfaceRaised)
            .border(1.dp, AppTheme.colors.borderDefault, CircleShape),
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
private fun DeleteSiteConfirmContent(
  documentCount: Int,
  folderCount: Int,
  isDeleting: () -> Boolean,
  onDelete:
    suspend context(SheetScope<Unit>)
    () -> Unit,
) {
  val isDeleting = isDeleting()
  var inputValue by remember { mutableStateOf("") }
  val confirmText = "$documentCount"
  val isConfirmed = documentCount == 0 || inputValue == confirmText

  SheetLayout(
    padding = SpaceDateDisplaySheetPadding,
    verticalSpacing = 12.dp,
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
        variant = if (isConfirmed) ButtonVariant.Danger else ButtonVariant.Secondary,
        enabled = isConfirmed && !isDeleting,
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
      text = deleteSiteBannerText(documentCount = documentCount, folderCount = folderCount),
      variant = AlertBannerVariant.Danger,
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

private fun deleteSiteBannerText(documentCount: Int, folderCount: Int): String {
  val totalCount = documentCount + folderCount

  return when {
    totalCount == 0 -> "비어있는 스페이스지만 삭제 후 복구할 수 없어요."
    documentCount > 0 && folderCount > 0 -> "${folderCount}개의 폴더와 ${documentCount}개의 문서가 함께 삭제돼요."
    documentCount > 0 -> "${documentCount}개의 문서가 함께 삭제돼요."
    else -> "${folderCount}개의 폴더가 함께 삭제돼요."
  }
}
