package co.typie.screen.settings.deleteuser

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.auth.AuthService
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.result.onErr
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.time.Duration.Companion.seconds

@Composable
fun DeleteUserScreen() {
  val model = viewModel { DeleteUserViewModel() }

  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current

  ProvideTopBar(center = { Text("회원 탈퇴", style = AppTheme.typography.title) })

  Screen { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().padding(contentPadding)) {
      Column(
        modifier =
          Modifier.weight(1f)
            .verticalScroll(scrollState)
            .padding(AppTheme.spacings.scrollBottomPadding),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        Column(
          modifier = Modifier.fillMaxWidth(),
          horizontalAlignment = Alignment.CenterHorizontally,
          verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
          Text("정말 탈퇴하시겠어요?", style = AppTheme.typography.title, textAlign = TextAlign.Center)

          Text(
            "탈퇴 전 아래 유의사항을 확인해주세요.",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textTertiary,
            textAlign = TextAlign.Center,
          )
        }

        Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
          Text(
            text = "유의사항",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textTertiary,
          )

          CardSurface(modifier = Modifier.fillMaxWidth()) {
            Column(
              modifier = Modifier.fillMaxWidth().padding(16.dp),
              verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
              listOf(
                  "- 작성한 모든 글과 데이터는 탈퇴와 함께 삭제되며 재가입시에도 복구할 수 없어요.",
                  "- 이용중인 스페이스 주소는 다시 이용할 수 없어요. 스페이스 주소를 다시 사용할 계획이라면, 탈퇴 전 기존 주소를 변경해주세요.",
                  "- 남은 이용권 기간은 탈퇴와 함께 소멸되며, 환불은 별도로 제공되지 않아요.",
                  "- 스토어에서 이용권을 구매했을 경우, 구독 취소 처리는 스토어 규정상 스토어 내 설정에서 직접 진행해야 해요.",
                )
                .forEach { item ->
                  Text(
                    text = item,
                    style = AppTheme.typography.caption,
                    color = AppTheme.colors.textSecondary,
                  )
                }
            }
          }
        }

        CardSurface(modifier = Modifier.fillMaxWidth()) {
          CardRow(
            onClick = { model.form.acknowledged.value = !model.form.acknowledged.value },
            spacing = 12.dp,
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 14.dp),
          ) {
            Box(
              modifier =
                Modifier.size(20.dp)
                  .clip(AppShapes.rounded(AppShapes.sm))
                  .background(
                    if (model.form.acknowledged.value) AppTheme.colors.brand
                    else AppTheme.colors.surfaceTinted,
                    AppShapes.rounded(AppShapes.sm),
                  )
                  .border(
                    1.dp,
                    if (model.form.acknowledged.value) AppTheme.colors.brand.copy(alpha = 0.72f)
                    else AppTheme.colors.borderDefault,
                    AppShapes.rounded(AppShapes.sm),
                  ),
              contentAlignment = Alignment.Center,
            ) {
              if (model.form.acknowledged.value) {
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

      ToastAnchor()

      Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(
          text = "탈퇴하기",
          variant = ButtonVariant.Danger,
          loading = model.isSubmitting,
          loadingText = "탈퇴하는 중...",
          onClick = {
            model
              .submit()
              .withDefaultExceptionHandler(toast)
              .onErr {
                when (it) {
                  is DeleteUserError.ValidationFailed -> toast.error(it.errorMessage)
                  is DeleteUserError.OverdueInvoicesExist ->
                    toast.error("미납된 결제가 있어 회원 탈퇴를 진행할 수 없어요. 결제 상태를 먼저 확인해주세요.")
                }
              }
              .onOk {
                toast.success("탈퇴가 완료되었어요. 이용해주셔서 감사합니다.", duration = 10.seconds)
                AuthService.logout()
              }
          },
        )

        Button(
          text = "타이피 계속 이용하기",
          variant = ButtonVariant.Secondary,
          enabled = !model.isSubmitting,
          onClick = { nav.pop() },
        )
      }
    }
  }
}
