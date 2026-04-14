package co.typie.screen.subscription.referral

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.PlatformModule
import co.typie.result.fold
import co.typie.ui.component.Button
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverPlacement
import co.typie.ui.component.popover.close
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun ReferralScreen() {
  val nav = Nav.current
  val model = viewModel { ReferralViewModel() }
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val clipboard = PlatformModule.clipboard
  val share = PlatformModule.share
  val scrollState = rememberScrollState()
  var inviteMessage by remember { mutableStateOf<String?>(null) }
  var isInviteLoading by remember { mutableStateOf(false) }

  suspend fun prefetchInviteMessage(showErrorToast: Boolean = true): String? {
    if (isInviteLoading) return inviteMessage

    isInviteLoading = true

    return try {
      val message =
        model
          .issueReferralInviteMessage()
          .fold(onOk = { it }, onErr = { null }, onException = { null })
      inviteMessage = message

      if (message == null && showErrorToast) {
        toast.show(ToastType.Error, "초대 링크를 불러올 수 없어요.")
      }

      message
    } finally {
      isInviteLoading = false
    }
  }

  suspend fun requireInviteMessage(): String? {
    return inviteMessage ?: prefetchInviteMessage(showErrorToast = true)
  }

  LaunchedEffect(Unit) { prefetchInviteMessage() }

  suspend fun copyLink() {
    val message = requireInviteMessage() ?: return

    val copied = clipboard.copy(message, "text/plain")
    if (copied) {
      toast.show(ToastType.Success, "초대 링크가 복사되었어요.")
    } else {
      toast.show(ToastType.Error, "초대 링크를 복사할 수 없어요.")
    }
  }

  suspend fun shareLink() {
    val message = requireInviteMessage() ?: return

    val shared = share.share(message)
    if (!shared) {
      toast.show(ToastType.Error, "초대 링크를 공유할 수 없어요.")
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("초대", style = AppTheme.typography.title) },
    trailing = { ReferralActionsMenu(onCopyLink = ::copyLink, onShareLink = ::shareLink) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  Screen(loading = model.query.state !is QueryState.Success) { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      val data = model.query.data
      val referrals = data.me.referrals
      val referralCount = referrals.size
      val compensatedCount = referrals.count { it.compensated }
      Text("초대", style = AppTheme.typography.display, modifier = Modifier.padding(top = 4.dp))

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(18.dp),
          verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
          Column(
            modifier = Modifier.fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(4.dp),
          ) {
            Text("친구 초대", style = AppTheme.typography.title)
            Text(
              "친구는 즉시 1달 무료 혜택을 받고, 첫 결제가 완료되면 나도 1달 무료 혜택을 받아요.",
              style = AppTheme.typography.body,
              color = AppTheme.colors.textTertiary,
            )
          }

          Column(
            modifier = Modifier.fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(12.dp),
          ) {
            ReferralBenefitSummaryRow(icon = Lucide.Ticket, label = "친구 혜택", value = "즉시 1달 무료")
            ReferralBenefitSummaryRow(icon = Lucide.Coins, label = "내 혜택", value = "첫 결제 후 1달 무료")
          }
        }
      }

      SectionTitle("초대 현황", modifier = Modifier.padding(top = 8.dp))

      Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        ReferralMetricCard(
          icon = Lucide.Users,
          label = "초대한 친구",
          value = "${referralCount}명",
          modifier = Modifier.weight(1f),
        )
        ReferralMetricCard(
          icon = Lucide.Gift,
          label = "받은 혜택",
          value = formatReferralBenefitText(compensatedCount),
          modifier = Modifier.weight(1f),
        )
      }

      SectionTitle("초대 혜택 안내", modifier = Modifier.padding(top = 8.dp))

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(18.dp),
          verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
          ReferralBulletPoint(
            "초대 링크를 통해 웹에서 가입하고, 웹에서 플랜을 가입해야 초대 혜택을 받을 수 있어요. 앱에서 가입하면 혜택을 받을 수 없어요."
          )
          ReferralBulletPoint(
            "친구가 초대 링크로 가입하면 친구는 즉시 FULL ACCESS 플랜 1개월에 해당하는 크레딧을 지급받아요. 지급받은 크레딧으로 바로 FULL ACCESS 플랜을 체험해볼 수 있어요."
          )
          ReferralBulletPoint(
            "친구가 크레딧을 통한 체험을 끝내고 첫 결제를 완료하면 나도 FULL ACCESS 플랜 1개월에 상응하는 크레딧을 지급받아요. 이 크레딧은 다음 FULL ACCESS 플랜 갱신시 자동으로 이용돼요."
          )
          ReferralBulletPoint("초대 횟수에는 제한이 없어요.")
        }
      }

      Spacer(Modifier.height(72.dp))

      Button(
        text = "초대 링크 복사",
        modifier = Modifier.padding(horizontal = 16.dp).padding(bottom = 16.dp),
        loading = isInviteLoading,
        loadingText = "불러오는 중...",
        enabled = !isInviteLoading,
        onClick = ::copyLink,
      )
    }
  }
}

@Composable
private fun ReferralActionsMenu(onCopyLink: suspend () -> Unit, onShareLink: suspend () -> Unit) {
  val scope = rememberCoroutineScope()

  Popover(
    placement = PopoverPlacement.BelowEnd,
    anchor = { TopBarButton(icon = Lucide.Ellipsis) },
    pane = {
      Column(modifier = Modifier.padding(PopoverDefaults.PanePadding)) {
        PopoverList(
          items =
            listOf(
              PopoverListItem(
                content = { ReferralActionItem(icon = Lucide.Copy, label = "초대 링크 복사") },
                onSelected = {
                  close()
                  scope.launch { onCopyLink() }
                },
              ),
              PopoverListItem(
                content = { ReferralActionItem(icon = Lucide.Share2, label = "공유하기") },
                onSelected = {
                  close()
                  scope.launch { onShareLink() }
                },
              ),
            )
        )
      }
    },
  )
}

private fun formatReferralBenefitText(compensatedCount: Int): String {
  return if (compensatedCount > 0) {
    "${compensatedCount}개월 무료"
  } else {
    "없음"
  }
}

@Composable
private fun ReferralActionItem(icon: IconData, label: String) {
  Row(
    verticalAlignment = Alignment.CenterVertically,
    modifier = Modifier.height(42.dp).padding(horizontal = 16.dp),
  ) {
    Icon(icon = icon, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textSecondary)
    Spacer(Modifier.size(12.dp))
    Text(label, style = AppTheme.typography.action, color = AppTheme.colors.textPrimary)
  }
}

@Composable
private fun ReferralMetricCard(
  icon: IconData,
  label: String,
  value: String,
  modifier: Modifier = Modifier,
) {
  CardSurface(modifier = modifier.fillMaxWidth(), shape = RoundedCornerShape(10.dp)) {
    Column(
      modifier = Modifier.fillMaxWidth().padding(16.dp),
      verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
      ReferralIconBadge(
        icon = icon,
        backgroundColor = AppTheme.colors.surfaceSunken,
        tint = AppTheme.colors.textSecondary,
      )

      Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(label, style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
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
private fun ReferralBenefitSummaryRow(icon: IconData, label: String, value: String) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(10.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    ReferralIconBadge(
      icon = icon,
      backgroundColor = AppTheme.colors.surfaceSunken,
      tint = AppTheme.colors.textSecondary,
      size = 32.dp,
      iconSize = 16.dp,
      cornerRadius = 10.dp,
      contentPadding = 8.dp,
    )

    Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
      Text(
        label,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textTertiary,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
      Text(
        value,
        style = AppTheme.typography.action,
        color = AppTheme.colors.textSecondary,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
      )
    }
  }
}

@Composable
private fun ReferralIconBadge(
  icon: IconData,
  backgroundColor: Color,
  tint: Color,
  modifier: Modifier = Modifier,
  size: androidx.compose.ui.unit.Dp = 36.dp,
  iconSize: androidx.compose.ui.unit.Dp = 18.dp,
  cornerRadius: androidx.compose.ui.unit.Dp = 12.dp,
  contentPadding: androidx.compose.ui.unit.Dp = 8.dp,
) {
  Box(
    modifier =
      modifier
        .size(size)
        .background(color = backgroundColor, shape = RoundedCornerShape(cornerRadius))
        .padding(contentPadding),
    contentAlignment = Alignment.Center,
  ) {
    Icon(icon = icon, modifier = Modifier.size(iconSize), tint = tint)
  }
}

@Composable
private fun ReferralBulletPoint(text: String) {
  Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.Top) {
    Box(
      modifier =
        Modifier.padding(top = 9.dp)
          .size(4.dp)
          .background(color = AppTheme.colors.textTertiary, shape = CircleShape)
    )
    Spacer(Modifier.size(8.dp))
    Text(
      text,
      modifier = Modifier.weight(1f),
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
    )
  }
}
