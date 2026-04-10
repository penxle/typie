package co.typie.screen.settings.update_profile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.imePadding
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.Img_image
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.platform.rememberFilePicker
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Img
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.time.Duration
import kotlinx.coroutines.launch
import co.typie.overlay.LocalToast
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun UpdateProfileScreen() {
  val nav = Nav.current
  val model = koinViewModel<UpdateProfileViewModel>()
  val toast = LocalToast.current
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val filePicker = rememberFilePicker { files ->
    val file = files.firstOrNull() ?: return@rememberFilePicker

    scope.launch {
      model.uploadAvatar(file).collect(
        onPending = { toast.show(ToastType.Loading, "프로필 사진 업로드 중...", Duration.ZERO) },
        onSettled = { result ->
          result
            .withDefaultExceptionHandler(toast)
            .onOk { avatarId ->
              toast.show(ToastType.Success, "프로필 사진이 업로드되었어요.")
              model.state.form.avatarId.setValue(avatarId)
            }
        },
      )
    }
  }

  ProvideTopBar(
    center = { Text("프로필 변경", style = AppTheme.typography.title) },
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    scrollState = scrollState,
    loading = model.query.state !is QueryState.Success,
    imeAware = true,
    bottomBar = {
      Button(
        text = "변경",
        modifier = Modifier
          .padding(horizontal = 16.dp)
          .padding(bottom = 16.dp),
        loading = model.isSubmitting,
        loadingText = "변경 중...",
        onClick = {
          scope.launch {
            model.submit()
              .withDefaultExceptionHandler(toast)
              .onOk {
                toast.show(ToastType.Success, "프로필이 변경되었어요.")
                nav.pop()
              }
          }
        },
      )
    },
  ) {
        Column(
          modifier = Modifier.fillMaxWidth(),
          horizontalAlignment = Alignment.CenterHorizontally,
          verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          ProfileAvatar(
            image = model.query.data.me.avatar.img_image,
            previewUrl = model.state.avatarPreviewUrl,
            onClick = { filePicker("image/*") },
          )

          Text(
            "프로필 사진",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textTertiary,
          )
        }

        Spacer(Modifier.height(32.dp))

        TextField(
          field = model.state.form.name,
          label = "닉네임",
          labelPosition = LabelPosition.Internal,
          onImeAction = {
            scope.launch {
              model.submit()
                .withDefaultExceptionHandler(toast)
                .onOk {
                  toast.show(ToastType.Success, "프로필이 변경되었어요.")
                  nav.pop()
                }
            }
          },
        )

        Spacer(Modifier.height(24.dp))
  }
}

@Composable
private fun ProfileAvatar(
  image: Img_image,
  previewUrl: String?,
  onClick: () -> Unit,
) {
  InteractionScope {
    Box(
      modifier = Modifier
        .clickable(onClick)
        .pressScale(),
    ) {
      Box(
        modifier = Modifier
          .size(104.dp)
          .clip(CircleShape)
          .background(AppTheme.colors.surfaceDefault)
          .border(1.dp, AppTheme.colors.borderDefault, CircleShape),
        contentAlignment = Alignment.Center,
      ) {
        if (previewUrl != null) {
          Img(
            url = previewUrl,
            modifier = Modifier.size(104.dp).clip(CircleShape),
          )
        } else {
          Img(
            image = image,
            modifier = Modifier.size(104.dp).clip(CircleShape),
          )
        }
      }

      Box(
        modifier = Modifier
          .align(Alignment.BottomEnd)
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
