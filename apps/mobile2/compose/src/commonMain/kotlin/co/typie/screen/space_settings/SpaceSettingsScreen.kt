package co.typie.screen.space_settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.imePadding
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.Img_image
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.rememberFilePicker
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Img
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun SpaceSettingsScreen() {
  val nav = Nav.current
  val model = koinViewModel<SpaceSettingsViewModel>()
  val scope = rememberCoroutineScope()
  LocalBottomSheetHost.current
  var showLastSiteAlert by remember { mutableStateOf(false) }

  val filePicker = rememberFilePicker { file ->
    if (file == null) return@rememberFilePicker
    scope.launch { model.uploadLogo(file) }
  }

  ProvideTopBar(
    center = { Text("스페이스 설정", style = AppTheme.typography.title) },
    trailing = { MoreMenu(model, showLastSiteAlert = { showLastSiteAlert = true }) },
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
  ) { contentPadding ->
    val data = model.query.data
    val hasSubscription = data.me.subscription != null

    Column(
      modifier = Modifier
        .fillMaxSize()
        .padding(contentPadding)
        .navigationBarsPadding()
        .imePadding(),
    ) {
      Column(
        modifier = Modifier.fillMaxWidth(),
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

      Spacer(Modifier.height(32.dp))

      TextField(
        field = model.state.form.name,
        label = "이름",
        labelPosition = LabelPosition.Internal,
        placeholder = "스페이스 이름",
      )

      TextField(
        field = model.state.form.slug,
        label = "주소",
        labelPosition = LabelPosition.Internal,
        placeholder = "스페이스 주소",
        enabled = hasSubscription,
        readOnly = !hasSubscription,
        suffix = {
          Text(
            ".${model.usersiteHost}",
            style = AppTheme.typography.body,
            color = AppTheme.colors.textSecondary,
          )
        },
      )

      if (!hasSubscription) {
        Text(
          "스페이스 주소 기능은 FULL ACCESS 플랜에서 사용할 수 있어요.",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
          modifier = Modifier.padding(start = 8.dp),
        )
      }

      Spacer(Modifier.weight(1f))

      Button(
        text = "저장",
        modifier = Modifier.padding(bottom = 16.dp),
        loading = model.state.isSubmitting,
        loadingText = "저장 중...",
        onClick = { model.submit { nav.pop() } },
      )
    }
  }

  if (showLastSiteAlert) {
    LastSiteAlertDialog(onDismiss = { showLastSiteAlert = false })
  }
}

@Composable
private fun MoreMenu(
  model: SpaceSettingsViewModel,
  showLastSiteAlert: () -> Unit,
) {
  val nav = Nav.current
  val scope = rememberCoroutineScope()
  val bottomSheetHost = LocalBottomSheetHost.current

  Popover(
    placement = PopoverPlacement.BelowEnd,
    anchor = { TopBarButton(icon = Lucide.Ellipsis) },
    pane = {
      Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
        PopoverList(
          items = listOf(
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
                  showLastSiteAlert()
                } else {
                  scope.launch {
                    val totalCount = data.site.documentCount + data.site.folderCount
                    bottomSheetHost.show {
                      DeleteSiteConfirmSheet(
                        totalCount = totalCount,
                        isDeleting = model.state.isDeleting,
                        onDelete = {
                          model.deleteSite {
                            dismiss()
                            nav.pop()
                          }
                        },
                      )
                    }
                  }
                }
              },
            ),
          ),
        )
      }
    },
  )
}

@Composable
private fun SpaceLogo(
  image: Img_image,
  previewUrl: String?,
  onClick: () -> Unit,
) {
  val logoShape = RoundedCornerShape(14.dp)

  InteractionScope {
    Box(
      modifier = Modifier
        .clickable(onClick)
        .pressScale(),
    ) {
      Box(
        modifier = Modifier
          .size(104.dp)
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
        modifier = Modifier
          .align(Alignment.BottomEnd)
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
private fun LastSiteAlertDialog(onDismiss: () -> Unit) {
  Dialog(onDismissRequest = onDismiss) {
    Column(
      modifier = Modifier
        .width(280.dp)
        .clip(RoundedCornerShape(16.dp))
        .background(AppTheme.colors.surfaceRaised),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      Column(
        modifier = Modifier.padding(start = 28.dp, end = 28.dp, top = 32.dp, bottom = 28.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        Text("스페이스를 삭제할 수 없어요", style = AppTheme.typography.title)
        Spacer(Modifier.height(6.dp))
        Text(
          "최소 1개의 스페이스가 필요해요.\n새 스페이스를 만든 후 삭제할 수 있어요.",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
        )
      }

      Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderSubtle))

      Box(
        modifier = Modifier
          .fillMaxWidth()
          .clickable { onDismiss() }
          .padding(vertical = 14.dp),
        contentAlignment = Alignment.Center,
      ) {
        Text("확인", style = AppTheme.typography.action)
      }
    }
  }
}

@Composable
fun BottomSheetScope<Unit>.DeleteSiteConfirmSheet(
  totalCount: Int,
  isDeleting: Boolean,
  onDelete: () -> Unit,
) {
  var inputValue by remember { mutableStateOf("") }
  val confirmText = "$totalCount"
  val isConfirmed = totalCount == 0 || inputValue == confirmText

  Column(
    modifier = Modifier.padding(horizontal = 16.dp),
    verticalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    Text("정말로 삭제하시겠어요?", style = AppTheme.typography.title)

    Text(
      "스페이스의 모든 글과 데이터가 삭제되며, 복구할 수 없어요.",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textSecondary,
    )

    if (totalCount > 0) {
      Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderSubtle))

      Text(
        "삭제를 진행하려면 스페이스와 함께 삭제되는 문서 수($totalCount)를 입력해주세요.",
        style = AppTheme.typography.caption,
      )

      TextField(
        value = inputValue,
        onValueChange = { inputValue = it },
        label = "문서 수",
        labelPosition = LabelPosition.None,
        placeholder = confirmText,
        keyboardType = KeyboardType.Number,
      )
    }

    Button(
      text = "삭제",
      variant = if (isConfirmed) ButtonVariant.Danger else ButtonVariant.Secondary,
      enabled = isConfirmed && !isDeleting,
      loading = isDeleting,
      loadingText = "삭제 중...",
      onClick = {
        if (isConfirmed) onDelete()
      },
    )
  }
}
