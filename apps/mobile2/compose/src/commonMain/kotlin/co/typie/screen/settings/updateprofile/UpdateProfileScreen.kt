package co.typie.screen.settings.updateprofile

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
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.fragment.Img_image
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.rememberFilePicker
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.Img
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.time.Duration
import kotlinx.coroutines.launch

@Composable
fun UpdateProfileScreen() {
  val model = viewModel { UpdateProfileViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current

  val filePicker = rememberFilePicker { files ->
    val file = files.firstOrNull() ?: return@rememberFilePicker

    scope.launch {
      model
        .uploadAvatar(file)
        .collect(
          onPending = { toast.show(ToastType.Loading, "프로필 사진 업로드 중...", Duration.ZERO) },
          onSettled = { result ->
            result.withDefaultExceptionHandler(toast).onOk { toast.success("프로필 사진이 업로드되었어요.") }
          },
        )
    }
  }

  fun submit() {
    scope.launch {
      model.submit().withDefaultExceptionHandler(toast).onOk {
        toast.success("프로필이 변경되었어요.")
        nav.pop()
      }
    }
  }

  ProvideTopBar(
    center = { Text("프로필 변경", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query) { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().padding(contentPadding)) {
      Column(modifier = Modifier.weight(1f).verticalScroll(scrollState)) {
        Column(
          modifier = Modifier.fillMaxWidth(),
          horizontalAlignment = Alignment.CenterHorizontally,
          verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          ProfileAvatar(
            image = model.query.data.me.avatar.img_image,
            previewUrl = model.avatarPreviewUrl,
            onClick = { filePicker("image/*") },
          )

          Text("프로필 사진", style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
        }

        Spacer(Modifier.height(32.dp))

        TextField(
          field = model.form.name,
          label = "닉네임",
          labelPosition = LabelPosition.Internal,
          onImeAction = { submit() },
        )
      }

      ToastAnchor()

      Button(
        text = "변경",
        loading = model.isSubmitting,
        loadingText = "변경 중...",
        onClick = { submit() },
      )
    }
  }
}

@Composable
private fun ProfileAvatar(image: Img_image, previewUrl: String?, onClick: () -> Unit) {
  InteractionScope {
    Box(modifier = Modifier.pressScale().clickable(onClick)) {
      Box(
        modifier =
          Modifier.size(104.dp)
            .clip(AppShapes.circle)
            .border(1.dp, AppTheme.colors.borderDefault, AppShapes.circle)
            .background(AppTheme.colors.surfaceDefault, AppShapes.circle),
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
            .size(36.dp)
            .border(1.dp, AppTheme.colors.borderDefault, AppShapes.circle)
            .background(AppTheme.colors.surfaceRaised, AppShapes.circle),
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
