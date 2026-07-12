package co.typie.screen.subscription.referral

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.excludeTop
import co.typie.ext.onlyTop
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.platform.PlatformModule
import co.typie.platform.ShareAnchor
import co.typie.platform.rememberShareAnchor
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun ReferralScreen() {
  val model = viewModel { ReferralViewModel() }
  val scrollState = rememberScrollState()
  val toast = LocalToast.current

  fun buildReferralInviteMessage(url: String): String = "📝 타이피 가입하고 한달 무료 혜택 받아가세요! $url"

  suspend fun copyLink() {
    model.issueReferralUrl().withDefaultExceptionHandler(toast).onOk {
      val message = buildReferralInviteMessage(it)
      val copied = PlatformModule.clipboard.copy(message, "text/plain")
      if (copied) {
        toast.success("초대 링크가 복사되었어요.")
      } else {
        toast.error("초대 링크를 복사할 수 없어요.")
      }
    }
  }

  suspend fun shareLink(anchor: ShareAnchor?) {
    model.issueReferralUrl().withDefaultExceptionHandler(toast).onOk {
      val message = buildReferralInviteMessage(it)
      val shared = PlatformModule.share.share(message, anchor)
      if (!shared) {
        toast.error("초대 링크를 공유할 수 없어요.")
      }
    }
  }

  ProvideTopBar(
    center = { Text("초대", style = AppTheme.typography.title) },
    trailing = { ActionsMenu(onCopyLink = { copyLink() }, onShareLink = { shareLink(it) }) },
    scrollOffset = scrollState.topBarScrollOffset(),
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
          verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
          Text("초대", style = AppTheme.typography.display)

          CardSurface(modifier = Modifier.fillMaxWidth()) {
            Column(
              modifier = Modifier.fillMaxWidth(),
              verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
              Column(
                modifier = Modifier.fillMaxWidth().padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
              ) {
                Column(
                  modifier = Modifier.fillMaxWidth(),
                  verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                  Text("친구 초대", style = AppTheme.typography.title)
                  Text(
                    "친구는 즉시 1달 무료 혜택을 받고, 첫 결제가 완료되면 나도 1달 무료 혜택을 받아요.",
                    style = AppTheme.typography.body,
                    color = AppTheme.colors.textMuted,
                  )
                }

                Column(
                  modifier = Modifier.fillMaxWidth(),
                  verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                  BenefitSummaryRow(icon = Lucide.Ticket, label = "친구 혜택", value = "즉시 1달 무료")
                  BenefitSummaryRow(icon = Lucide.Coins, label = "내 혜택", value = "첫 결제 후 1달 무료")
                }
              }
            }
          }

          SectionTitle("초대 현황", modifier = Modifier.padding(top = 8.dp))

          Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
          ) {
            MetricCard(
              icon = Lucide.Users,
              label = "초대한 친구",
              value = "${model.query.data.me.referrals.size}명",
              modifier = Modifier.weight(1f),
            )

            MetricCard(
              icon = Lucide.Gift,
              label = "받은 혜택",
              value =
                model.query.data.me.referrals
                  .count { it.compensated }
                  .takeIf { it > 0 }
                  ?.let { "${it}개월 무료" } ?: "없음",
              modifier = Modifier.weight(1f),
            )
          }

          SectionTitle("초대 혜택 안내", modifier = Modifier.padding(top = 8.dp))

          CardSurface(modifier = Modifier.fillMaxWidth()) {
            Column(
              modifier = Modifier.fillMaxWidth().padding(16.dp),
              verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
              BulletPoint(
                "초대 링크를 통해 웹에서 가입하고, 웹에서 플랜을 가입해야 초대 혜택을 받을 수 있어요. 앱에서 가입하면 혜택을 받을 수 없어요."
              )
              BulletPoint(
                "친구가 초대 링크로 가입하면 친구는 즉시 FULL ACCESS 플랜 1개월에 해당하는 크레딧을 지급받아요. 지급받은 크레딧으로 바로 FULL ACCESS 플랜을 체험해볼 수 있어요."
              )
              BulletPoint(
                "친구가 크레딧을 통한 체험을 끝내고 첫 결제를 완료하면 나도 FULL ACCESS 플랜 1개월에 상응하는 크레딧을 지급받아요. 이 크레딧은 다음 FULL ACCESS 플랜 갱신시 자동으로 이용돼요."
              )
              BulletPoint("초대 횟수에는 제한이 없어요.")
            }
          }
        }
      }

      ToastAnchor()

      Button(text = "초대 링크 복사", onClick = { copyLink() })
    }
  }
}

@Composable
private fun ActionsMenu(
  onCopyLink: suspend () -> Unit,
  onShareLink: suspend (ShareAnchor?) -> Unit,
) {
  val scope = rememberCoroutineScope()
  val shareAnchor = rememberShareAnchor()

  PopoverMenu(anchor = { TopBarButton(icon = Lucide.Ellipsis, modifier = shareAnchor.modifier) }) {
    item(icon = Lucide.Copy, label = "초대 링크 복사") { scope.launch { onCopyLink() } }
    item(icon = Lucide.Share2, label = "초대 링크 공유") {
      scope.launch { onShareLink(shareAnchor.value) }
    }
  }
}

@Composable
private fun MetricCard(
  icon: IconData,
  label: String,
  value: String,
  modifier: Modifier = Modifier,
) {
  CardSurface(modifier = modifier.fillMaxWidth(), shape = AppShapes.rounded(AppShapes.md)) {
    Column(
      modifier = Modifier.fillMaxWidth().padding(16.dp),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Box(
        modifier =
          Modifier.size(36.dp)
            .background(
              color = AppTheme.colors.surfaceInset,
              shape = AppShapes.rounded(AppShapes.md),
            ),
        contentAlignment = Alignment.Center,
      ) {
        Icon(icon = icon, modifier = Modifier.size(20.dp), tint = AppTheme.colors.textMuted)
      }

      Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(label, style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

        Text(
          value,
          style = AppTheme.typography.heading,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }
    }
  }
}

@Composable
private fun BenefitSummaryRow(icon: IconData, label: String, value: String) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(12.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Box(
      modifier =
        Modifier.size(32.dp)
          .background(
            color = AppTheme.colors.surfaceInset,
            shape = AppShapes.rounded(AppShapes.md),
          ),
      contentAlignment = Alignment.Center,
    ) {
      Icon(icon = icon, modifier = Modifier.size(16.dp), tint = AppTheme.colors.textMuted)
    }

    Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
      Text(
        label,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textMuted,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )

      Text(
        value,
        style = AppTheme.typography.action,
        color = AppTheme.colors.textMuted,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }
  }
}

@Composable
private fun BulletPoint(text: String) {
  Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.Top) {
    Box(
      modifier =
        Modifier.padding(top = 8.dp)
          .size(4.dp)
          .background(color = AppTheme.colors.textMuted, shape = AppShapes.circle)
    )

    Spacer(Modifier.size(8.dp))

    Text(
      text,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textMuted,
    )
  }
}
