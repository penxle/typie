package co.typie.screen.more.more

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
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
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.toLocalDate
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.SubscriptionServiceState
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.generated.resources.Res
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.shell.MainBottomBarActionButton
import co.typie.shell.MainBottomBarPill
import co.typie.ui.component.ActivityGrid
import co.typie.ui.component.ActivityGridChange
import co.typie.ui.component.ActivityGridHeight
import co.typie.ui.component.CardActionTile
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.skeleton.SkeletonBone
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun MoreScreen() {
  val nav = Nav.current
  val uriHandler = LocalUriHandler.current
  val sheet = LocalSheet.current
  val scope = rememberCoroutineScope()
  val model = viewModel { MoreViewModel() }
  val subscriptionState = SubscriptionService.state

  val scrollState = rememberScrollState()

  ProvideTopBar(
    leading = null,
    center = { Text("더 보기", style = AppTheme.typography.title) },
    trailing = { TopBarButton(Lucide.Settings, onClick = { nav.navigate(Route.Settings) }) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  ProvideBottomBar(pill = { MainBottomBarPill() }, action = { MainBottomBarActionButton() })

  Screen(query = model.query) { contentPadding ->
    Box(Modifier.fillMaxSize()) {
      Column(
        modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        val data = model.query.data
        val subscription = (subscriptionState as? SubscriptionServiceState.Subscribed)?.subscription
        val subscriptionActionLabel =
          when (subscriptionState) {
            is SubscriptionServiceState.Subscribed -> "이용권 정보"
            is SubscriptionServiceState.NotSubscribed -> "구매하기"
            is SubscriptionServiceState.Unknown -> "확인 중"
          }

        val activityChanges =
          data.me.characterCountChanges.map { change ->
            ActivityGridChange(date = change.date.toLocalDate(), additions = change.additions)
          }

        Skeleton.Keep { Text("더 보기", style = AppTheme.typography.display) }

        CardSurface(modifier = Modifier.fillMaxWidth()) {
          CardRow(
            onClick = { nav.navigate(Route.UpdateProfile) },
            contentPadding = PaddingValues(horizontal = 18.dp, vertical = 18.dp),
            spacing = 16.dp,
          ) {
            Img(
              image = data.me.avatar.img_image,
              modifier = Modifier.clip(AppShapes.circle).size(72.dp),
            )

            Column(
              modifier = Modifier.weight(1f),
              verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
              Text(
                data.me.name,
                style = AppTheme.typography.heading,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )

              Text(
                data.me.email,
                style = AppTheme.typography.action,
                color = AppTheme.colors.textTertiary,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            }

            Skeleton.Ignore {
              Icon(
                icon = Lucide.ChevronRight,
                modifier = Modifier.size(16.dp),
                tint = AppTheme.colors.textTertiary,
              )
            }
          }
        }

        CardSurface(modifier = Modifier.fillMaxWidth()) {
          Column {
            Column(
              modifier = Modifier.padding(horizontal = 16.dp, vertical = 16.dp),
              verticalArrangement = Arrangement.spacedBy(3.dp),
            ) {
              Text("나의 글쓰기 활동", style = AppTheme.typography.label)

              Text(
                "지난 1년 동안의 기록이에요",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
              )
            }

            Skeleton.Replace(
              replacement = {
                SkeletonBone(
                  Modifier.padding(horizontal = 16.dp).fillMaxWidth().height(ActivityGridHeight)
                )
              }
            ) {
              ActivityGrid(
                changes = activityChanges,
                modifier = Modifier.fillMaxWidth(),
                onVerticalScrollDelta = { delta -> scrollState.dispatchRawDelta(delta) },
              )
            }

            CardDivider()

            CardRow(onClick = { nav.navigate(Route.Stats) }) {
              Skeleton.Unite {
                Icon(
                  icon = Lucide.BarChart3,
                  modifier = Modifier.size(20.dp),
                  tint = AppTheme.colors.textSecondary,
                )

                Text("통계", style = AppTheme.typography.label)
              }

              Spacer(Modifier.weight(1f))

              Skeleton.Ignore {
                Icon(
                  icon = Lucide.ChevronRight,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textTertiary,
                )
              }
            }
          }
        }

        CardSurface(modifier = Modifier.fillMaxWidth()) {
          Column {
            CardRow(
              onClick = {
                when (subscriptionState) {
                  is SubscriptionServiceState.Subscribed -> nav.navigate(Route.CurrentPlan)
                  is SubscriptionServiceState.NotSubscribed -> nav.navigate(Route.EnrollPlan)
                  is SubscriptionServiceState.Unknown -> {}
                }
              }
            ) {
              Skeleton.Unite {
                Icon(
                  icon = Lucide.CreditCard,
                  modifier = Modifier.size(18.dp).align(Alignment.Top),
                  tint = AppTheme.colors.textSecondary,
                )

                Column(
                  modifier = Modifier.weight(1f),
                  verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                  Text("현재 이용권", style = AppTheme.typography.label)

                  Text(
                    subscription?.planName ?: "타이피 BASIC ACCESS",
                    style = AppTheme.typography.caption,
                    color = AppTheme.colors.textTertiary,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                  )
                }
              }

              Row(
                modifier = Modifier.align(Alignment.Top),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(2.dp),
              ) {
                Skeleton.Unite {
                  Text(
                    subscriptionActionLabel,
                    style = AppTheme.typography.caption,
                    color = AppTheme.colors.textTertiary,
                  )

                  Icon(
                    icon = Lucide.ChevronRight,
                    modifier = Modifier.size(14.dp),
                    tint = AppTheme.colors.textTertiary,
                  )
                }
              }
            }
          }
        }

        CardSurface(modifier = Modifier.fillMaxWidth()) {
          CardRow(onClick = { nav.navigate(Route.Settings) }) {
            Skeleton.Unite {
              Icon(
                icon = Lucide.Settings,
                modifier = Modifier.size(20.dp),
                tint = AppTheme.colors.textSecondary,
              )

              Text("설정", style = AppTheme.typography.label)
            }

            Spacer(Modifier.weight(1f))

            Skeleton.Ignore {
              Icon(
                icon = Lucide.ChevronRight,
                modifier = Modifier.size(16.dp),
                tint = AppTheme.colors.textTertiary,
              )
            }
          }
        }

        Spacer(Modifier.height(8.dp))

        Skeleton.Keep { SectionTitle("도움 및 링크") }

        Row(
          modifier = Modifier.fillMaxWidth(),
          horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
          CardActionTile(
            onClick = { uriHandler.openUri("https://penxle.channel.io/home") },
            modifier = Modifier.weight(1f),
          ) {
            Row(
              horizontalArrangement = Arrangement.SpaceBetween,
              verticalAlignment = Alignment.CenterVertically,
              modifier = Modifier.fillMaxWidth(),
            ) {
              Skeleton.Ignore {
                Icon(
                  icon = Lucide.Headphones,
                  modifier = Modifier.size(20.dp),
                  tint = AppTheme.colors.textSecondary,
                )

                Icon(
                  icon = Lucide.ExternalLink,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textTertiary,
                )
              }
            }

            Text("고객센터", style = AppTheme.typography.label)
          }

          CardActionTile(
            onClick = { scope.launch { sheet.present { FeedbackContent() } } },
            modifier = Modifier.weight(1f),
          ) {
            Row(
              horizontalArrangement = Arrangement.SpaceBetween,
              verticalAlignment = Alignment.CenterVertically,
              modifier = Modifier.fillMaxWidth(),
            ) {
              Skeleton.Ignore {
                Icon(
                  icon = Lucide.MessageSquare,
                  modifier = Modifier.size(20.dp),
                  tint = AppTheme.colors.textSecondary,
                )

                Spacer(Modifier.size(16.dp))
              }
            }

            Text("의견 보내기", style = AppTheme.typography.label)
          }
        }

        CardSurface(modifier = Modifier.fillMaxWidth()) {
          Column {
            if (subscriptionState is SubscriptionServiceState.Subscribed) {
              CardRow(onClick = { uriHandler.openUri("https://typie.link/community") }) {
                Skeleton.Unite {
                  Img(
                    url = Res.getUri("files/brands/discord.svg"),
                    modifier = Modifier.size(20.dp),
                    color = AppTheme.colors.textSecondary,
                  )

                  Text("타이피 유저 커뮤니티", style = AppTheme.typography.label)
                }

                Spacer(Modifier.weight(1f))

                Skeleton.Ignore {
                  Icon(
                    icon = Lucide.ExternalLink,
                    modifier = Modifier.size(16.dp),
                    tint = AppTheme.colors.textTertiary,
                  )
                }
              }

              CardDivider()
            }

            CardRow(onClick = { uriHandler.openUri("https://x.com/typieofficial") }) {
              Skeleton.Unite {
                Img(
                  url = Res.getUri("files/brands/x.svg"),
                  modifier = Modifier.size(18.dp),
                  color = AppTheme.colors.textSecondary,
                )

                Text("타이피 공식 X", style = AppTheme.typography.label)
              }

              Spacer(Modifier.weight(1f))

              Skeleton.Ignore {
                Icon(
                  icon = Lucide.ExternalLink,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textTertiary,
                )
              }
            }

            CardDivider()

            CardRow(onClick = { uriHandler.openUri("https://typie.co/changelog") }) {
              Skeleton.Unite {
                Icon(
                  icon = Lucide.Newspaper,
                  modifier = Modifier.size(20.dp),
                  tint = AppTheme.colors.textSecondary,
                )

                Text("업데이트 노트", style = AppTheme.typography.label)
              }

              Spacer(Modifier.weight(1f))

              Skeleton.Ignore {
                Icon(
                  icon = Lucide.ExternalLink,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textTertiary,
                )
              }
            }
          }
        }

        Spacer(Modifier.height(140.dp))
      }

      ToastAnchor(
        modifier =
          Modifier.align(Alignment.BottomCenter)
            .navigationBarsPadding()
            .padding(bottom = BottomBarDefaults.BarAreaHeight)
      )
    }
  }
}
