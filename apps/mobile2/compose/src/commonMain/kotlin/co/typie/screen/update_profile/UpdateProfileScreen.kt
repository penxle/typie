package co.typie.screen.update_profile

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
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.graphql.QueryState
import co.typie.graphql.fragment.Img_image
import co.typie.icons.Lucide
import co.typie.media.rememberImagePicker
import co.typie.navigation.Nav
import co.typie.ui.component.Button
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import coil3.compose.AsyncImage
import kotlinx.coroutines.launch
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun UpdateProfileScreen() {
  val nav = Nav.current
  val model = koinViewModel<UpdateProfileViewModel>()
  val scope = rememberCoroutineScope()

  ProvideTopBar(
    center = { Text("프로필 변경", style = AppTheme.typography.title) },
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceSubtle,
  ) { contentPadding ->
    val data = model.query.data

    val imagePicker = rememberImagePicker { image ->
      if (image == null || model.state.isUploadingAvatar) {
        return@rememberImagePicker
      }

      scope.launch {
        val avatarId = model.uploadAvatar(image) ?: return@launch
        model.state.form.avatarId.setValue(avatarId)
      }
    }

    Column(
      modifier = Modifier
        .fillMaxSize()
        .padding(contentPadding)
        .navigationBarsPadding()
        .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
      Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        ProfileAvatar(
          image = data.me.avatar.img_image,
          previewUrl = model.state.avatarPreviewUrl,
          uploading = model.state.isUploadingAvatar,
          onClick = imagePicker,
        )

        Text(
          if (model.state.isUploadingAvatar) "프로필 사진 업로드 중..." else "프로필 사진",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textFaint,
        )
      }

      Spacer(Modifier.height(32.dp))

      TextField(
        field = model.state.form.name,
        label = "닉네임",
      )

      Spacer(Modifier.weight(1f))

      Button(
        text = "변경",
        modifier = Modifier.padding(bottom = 16.dp),
        enabled = !model.state.isUploadingAvatar,
        loading = model.state.isSubmitting,
        loadingText = "변경 중...",
        onClick = { model.submit { nav.pop() } },
      )
    }
  }
}

@Composable
private fun ProfileAvatar(
  image: Img_image?,
  previewUrl: String?,
  uploading: Boolean,
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
          AsyncImage(
            model = previewUrl,
            contentDescription = null,
            modifier = Modifier
              .size(104.dp)
              .clip(CircleShape),
            contentScale = ContentScale.Crop,
          )
        } else if (image != null) {
          Img(
            image = image,
            modifier = Modifier.size(104.dp).clip(CircleShape),
          )
        } else {
          Icon(
            icon = Lucide.UserRound,
            modifier = Modifier.size(36.dp),
            tint = AppTheme.colors.textDisabled,
          )
        }
      }

      Box(
        modifier = Modifier
          .align(Alignment.BottomEnd)
          .size(36.dp)
          .clip(CircleShape)
          .background(AppTheme.colors.surfaceElevated)
          .border(1.dp, AppTheme.colors.borderDefault, CircleShape),
        contentAlignment = Alignment.Center,
      ) {
        Icon(
          icon = Lucide.Camera,
          modifier = Modifier.size(18.dp),
          tint = if (uploading) AppTheme.colors.textDisabled else AppTheme.colors.textSubtle,
        )
      }
    }
  }
}
