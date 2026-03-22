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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.blob.BlobUploader
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.form.FieldState
import co.typie.form.FormField
import co.typie.form.rememberFormState
import co.typie.graphql.GraphQLContent
import co.typie.graphql.UpdateProfileScreen_Query
import co.typie.graphql.fragment.Img_image
import co.typie.graphql.rememberQuery
import co.typie.icons.Lucide
import co.typie.media.rememberImagePicker
import co.typie.navigation.Nav
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.ui.component.Button
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import com.apollographql.apollo.ApolloClient
import coil3.compose.AsyncImage
import kotlinx.coroutines.launch
import org.koin.compose.koinInject

@Composable
fun UpdateProfileScreen() {
  val nav = Nav.current
  val apolloClient = koinInject<ApolloClient>()
  val blobUploader = koinInject<BlobUploader>()
  val toast = koinInject<Toast>()
  val viewModel = viewModel { UpdateProfileViewModel(apolloClient, blobUploader) }
  val query = rememberQuery(UpdateProfileScreen_Query())
  val scope = rememberCoroutineScope()

  ProvideTopBar(
    center = { Text("프로필 변경", style = AppTheme.typography.title) },
  )

  LaunchedEffect(viewModel.state.completedSubmissionCount) {
    if (viewModel.state.completedSubmissionCount > 0) {
      nav.pop()
    }
  }

  LaunchedEffect(viewModel.state.errorMessage) {
    val message = viewModel.state.errorMessage ?: return@LaunchedEffect
    toast.show(ToastType.Error, message)
    viewModel.consumeError()
  }

  Screen { contentPadding ->
    GraphQLContent(query) { data ->
      val form = rememberFormState(data.me.id, data.me.name, data.me.avatar.id) {
        UpdateProfileForm(
          initialName = data.me.name,
          initialAvatarId = data.me.avatar.id,
        )
      }
      val imagePicker = rememberImagePicker { image ->
        if (image == null || viewModel.state.isUploadingAvatar) {
          return@rememberImagePicker
        }

        scope.launch {
          val avatarId = viewModel.uploadAvatar(image) ?: return@launch
          form.avatarId.setValue(avatarId)
        }
      }
      val isUploadingAvatar = viewModel.state.isUploadingAvatar
      val canSubmit = form.isDirty && !form.isProcessing && !isUploadingAvatar

      Column(
        modifier = Modifier
          .fillMaxSize()
          .background(AppTheme.colors.surfaceSubtle)
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
            previewUrl = viewModel.state.avatarPreviewUrl,
            uploading = viewModel.state.isUploadingAvatar,
            onClick = imagePicker,
          )
          Text(
            if (viewModel.state.isUploadingAvatar) "프로필 사진 업로드 중..." else "프로필 사진",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textFaint,
          )
        }

        Spacer(Modifier.height(32.dp))

        ProfileNameField(
          field = form.name,
          onDone = {
            form.submit(scope) {
              viewModel.submit(
                name = form.name.value.trim(),
                avatarId = form.avatarId.value,
              )
            }
          },
        )

        Spacer(Modifier.weight(1f))

        Button(
          text = "변경",
          onClick = {
            form.submit(scope) {
              viewModel.submit(
                name = form.name.value.trim(),
                avatarId = form.avatarId.value,
              )
            }
          },
          loading = form.isProcessing,
          loadingText = "변경 중...",
          enabled = canSubmit,
          modifier = Modifier.padding(bottom = 16.dp),
        )
      }
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
        .pressScale(0.98f),
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
            size = 104.dp,
            modifier = Modifier.clip(CircleShape),
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

@Composable
private fun ProfileNameField(
  field: FieldState<String>,
  onDone: () -> Unit,
) {
  val shape = RoundedCornerShape(12.dp)

  FormField(field = field) { nameField ->
    val borderColor =
      if (nameField.errors.isNotEmpty()) AppTheme.colors.borderDanger else AppTheme.colors.borderDefault

    Column {
      Text(
        "닉네임",
        style = AppTheme.typography.caption,
      )
      Spacer(Modifier.height(8.dp))
      BasicTextField(
        value = nameField.value,
        onValueChange = nameField::setValue,
        textStyle = AppTheme.typography.action.copy(color = AppTheme.colors.textDefault),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
        keyboardActions = KeyboardActions(onDone = { onDone() }),
        singleLine = true,
        decorationBox = { innerTextField ->
          Box(
            modifier = Modifier
              .fillMaxWidth()
              .height(48.dp)
              .border(1.dp, borderColor, shape)
              .background(AppTheme.colors.surfaceDefault, shape)
              .padding(horizontal = 16.dp),
            contentAlignment = Alignment.CenterStart,
          ) {
            if (nameField.value.isEmpty()) {
              Text(
                "닉네임",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textDisabled,
              )
            }
            innerTextField()
          }
        },
      )
    }
  }
}
