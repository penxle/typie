package co.typie.screen.delete_user

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.auth.AuthService
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

@Composable
fun DeleteUserScreen() {
  val nav = Nav.current
  val model = koinViewModel<DeleteUserViewModel>()
  val authService = koinInject<AuthService>()
  val toast = LocalToast.current
  val scrollState = rememberScrollState()
  var isAcknowledged by remember { mutableStateOf(false) }

  suspend fun submit() {
    val validationMessage = deleteUserValidationMessage(isAcknowledged = isAcknowledged)
    if (validationMessage != null) {
      toast.show(ToastType.Error, validationMessage)
      return
    }

    toast.show(ToastType.Loading, "탈퇴하는 중...")
    model.deleteUser()
      .withDefaultExceptionHandler(toast)
      .onOk {
        toast.dismiss()
        authService.clearSession()
      }
  }

  ProvideTopBar(
    center = { Text("회원 탈퇴", style = AppTheme.typography.title) },
  )

  Screen(
    scrollState = scrollState,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
    bottomBar = {
      Column(
        modifier = Modifier
          .padding(horizontal = 16.dp)
          .padding(bottom = 16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        Button(
          text = "탈퇴하기",
          variant = ButtonVariant.Danger,
          loading = model.isSubmitting,
          loadingText = "탈퇴하는 중...",
          onClick = { submit() },
        )

        Button(
          text = "타이피 계속 이용하기",
          variant = ButtonVariant.Secondary,
          enabled = !model.isSubmitting,
          onClick = { nav.pop() },
        )
      }
    },
  ) {
        Column(
          modifier = Modifier
            .fillMaxWidth(),
          horizontalAlignment = Alignment.CenterHorizontally,
          verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
          Text(
            "정말 탈퇴하시겠어요?",
            style = AppTheme.typography.title.copy(textAlign = TextAlign.Center),
          )

          Text(
            "탈퇴 전 아래 유의사항을 확인해주세요.",
            style = AppTheme.typography.caption.copy(textAlign = TextAlign.Center),
            color = AppTheme.colors.textTertiary,
          )
        }

        Column(
          verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          SectionTitle("유의사항")

          CardSurface(
            modifier = Modifier.fillMaxWidth(),
          ) {
            Column(
              modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
              verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
              deleteUserNoticeItems().forEach { item ->
                Text(
                  text = item,
                  style = AppTheme.typography.caption,
                  color = AppTheme.colors.textSecondary,
                )
              }
            }
          }
        }

        DeleteUserAcknowledgeRow(
          checked = isAcknowledged,
          onToggle = { isAcknowledged = !isAcknowledged },
        )
  }
}

@Composable
private fun DeleteUserAcknowledgeRow(
  checked: Boolean,
  onToggle: () -> Unit,
) {
  CardSurface(
    modifier = Modifier.fillMaxWidth(),
  ) {
    CardRow(
      onClick = { onToggle() },
      spacing = 12.dp,
      contentPadding = PaddingValues(horizontal = 16.dp, vertical = 14.dp),
    ) {
      Box(
        modifier = Modifier
          .size(20.dp)
          .clip(RoundedCornerShape(6.dp))
          .background(
            if (checked) AppTheme.colors.brand else AppTheme.colors.surfaceTinted,
            RoundedCornerShape(6.dp),
          )
          .border(
            1.dp,
            if (checked) AppTheme.colors.brand.copy(alpha = 0.72f) else AppTheme.colors.borderDefault,
            RoundedCornerShape(6.dp),
          ),
        contentAlignment = Alignment.Center,
      ) {
        if (checked) {
          Icon(
            icon = Lucide.Check,
            modifier = Modifier.size(14.dp),
            tint = AppTheme.colors.textOnBrand,
          )
        }
      }

      Text(
        text = "위 유의사항을 모두 확인했어요",
        style = AppTheme.typography.action,
        modifier = Modifier.weight(1f),
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }
  }
}
