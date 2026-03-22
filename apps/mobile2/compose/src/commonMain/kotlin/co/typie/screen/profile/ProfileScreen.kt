package co.typie.screen.profile

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
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.auth.AuthService
import co.typie.ext.clickable
import co.typie.datetime.toInstantOrNull
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.GraphQLContent
import co.typie.graphql.ProfileScreen_Query
import co.typie.graphql.rememberQuery
import co.typie.icons.Lucide
import co.typie.generated.resources.Res
import co.typie.navigation.Nav
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.route.Route
import co.typie.ui.component.ActivityGrid
import co.typie.ui.component.ActivityGridChange
import co.typie.ui.component.CardActionTile
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Img
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ThemeMode
import coil3.compose.AsyncImage
import kotlinx.datetime.LocalDate
import kotlinx.datetime.TimeZone
import kotlinx.datetime.toLocalDateTime
import kotlinx.coroutines.launch
import org.koin.compose.koinInject

@Composable
fun ProfileScreen() {
  val authService = koinInject<AuthService>()
  val nav = Nav.current
  val toast = koinInject<Toast>()
  val query = rememberQuery(ProfileScreen_Query())
  val scrollState = rememberScrollState()
  val uriHandler = LocalUriHandler.current
  val themeMode = LocalThemeMode.current
  val scope = rememberCoroutineScope()

  ProvideTopBar(
    leading = null,
    center = { Text("프로필", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen { contentPadding ->
    GraphQLContent(query) { data ->
      Box(
        modifier = Modifier
          .fillMaxSize()
          .background(AppTheme.colors.surfaceSubtle),
      ) {
        Column(
          modifier = Modifier
            .fillMaxSize()
            .verticalScroll(scrollState)
            .padding(contentPadding)
            .navigationBarsPadding(),
          verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
          val hasSubscription = data.me.subscription != null
          val subscriptionName = data.me.subscription?.plan?.name ?: "타이피 BASIC ACCESS"
          val subscriptionActionLabel = if (hasSubscription) "이용권 정보" else "구매하기"
          val activityChanges = data.me.characterCountChanges.mapNotNull { change ->
            change.date.toString().toActivityGridDateOrNull()?.let { date ->
              ActivityGridChange(
                date = date,
                additions = change.additions,
              )
            }
          }

          Text("프로필", style = AppTheme.typography.display)

          CardSurface(
            modifier = Modifier
              .fillMaxWidth(),
          ) {
            CardRow(
              onClick = { nav.navigate(Route.UpdateProfile) },
              contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 18.dp, vertical = 18.dp),
              spacing = 16.dp,
            ) {
              Img(
                image = data.me.avatar.img_image,
                size = 72.dp,
                modifier = Modifier.clip(CircleShape),
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
                  color = AppTheme.colors.textFaint,
                  maxLines = 1,
                  overflow = TextOverflow.Ellipsis,
                )
              }
              Icon(
                icon = Lucide.ChevronRight,
                modifier = Modifier.size(16.dp),
                tint = AppTheme.colors.textFaint,
              )
            }
          }

          CardSurface(
            modifier = Modifier.fillMaxWidth(),
          ) {
            Column {
              Column(
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 16.dp),
                verticalArrangement = Arrangement.spacedBy(3.dp),
              ) {
                Text(
                  "나의 글쓰기 활동",
                  style = AppTheme.typography.title,
                )
                Text(
                  "지난 1년 동안의 기록이에요",
                  style = AppTheme.typography.caption,
                  color = AppTheme.colors.textFaint,
                )
              }

              ActivityGrid(
                changes = activityChanges,
                modifier = Modifier.fillMaxWidth(),
                onVerticalScrollDelta = { delta -> scrollState.dispatchRawDelta(delta) },
              )

              CardDivider()

              CardRow(
                onClick = {},
              ) {
                Icon(
                  icon = Lucide.BarChart3,
                  modifier = Modifier.size(20.dp),
                  tint = AppTheme.colors.textSubtle,
                )
                Text(
                  "통계",
                  modifier = Modifier.weight(1f),
                  style = AppTheme.typography.title,
                )
                Icon(
                  icon = Lucide.ChevronRight,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textFaint,
                )
              }
            }
          }

          CardSurface(
            modifier = Modifier.fillMaxWidth(),
          ) {
            Column {
              CardRow(
                onClick = {},
              ) {
                Icon(
                  icon = Lucide.CreditCard,
                  modifier = Modifier
                    .size(18.dp)
                    .align(Alignment.Top),
                  tint = AppTheme.colors.textSubtle,
                )
                Column(
                  modifier = Modifier.weight(1f),
                  verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                  Text(
                    "현재 이용권",
                    style = AppTheme.typography.title,
                  )
                  Text(
                    subscriptionName,
                    style = AppTheme.typography.action,
                    color = AppTheme.colors.textFaint,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                  )
                }
                Row(
                  modifier = Modifier.align(Alignment.Top),
                  verticalAlignment = Alignment.CenterVertically,
                  horizontalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                  Text(
                    subscriptionActionLabel,
                    style = AppTheme.typography.action,
                    color = AppTheme.colors.textFaint,
                  )
                  Icon(
                    icon = Lucide.ChevronRight,
                    modifier = Modifier.size(14.dp),
                    tint = AppTheme.colors.textFaint,
                  )
                }
              }

              CardDivider()

              CardRow(
                onClick = {},
              ) {
                Icon(
                  icon = Lucide.Gift,
                  modifier = Modifier.size(20.dp),
                  tint = AppTheme.colors.textSubtle,
                )
                Text(
                  "초대",
                  modifier = Modifier.weight(1f),
                  style = AppTheme.typography.title,
                )
                Icon(
                  icon = Lucide.ChevronRight,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textFaint,
                )
              }
            }
          }

          CardSurface(
            modifier = Modifier.fillMaxWidth(),
          ) {
            CardRow(
              onClick = {},
            ) {
              Icon(
                icon = Lucide.Settings,
                modifier = Modifier.size(20.dp),
                tint = AppTheme.colors.textSubtle,
              )
              Text(
                "설정",
                modifier = Modifier.weight(1f),
                style = AppTheme.typography.title,
              )
              Icon(
                icon = Lucide.ChevronRight,
                modifier = Modifier.size(16.dp),
                tint = AppTheme.colors.textFaint,
              )
            }
          }

          Spacer(Modifier.height(8.dp))

          SectionTitle("도움 및 링크")

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
                Icon(
                  icon = Lucide.Headphones,
                  modifier = Modifier.size(20.dp),
                  tint = AppTheme.colors.textSubtle,
                )
                Icon(
                  icon = Lucide.ExternalLink,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textFaint,
                )
              }

              Text(
                "고객센터",
                style = AppTheme.typography.title,
              )
            }
            CardActionTile(
              onClick = {},
              modifier = Modifier.weight(1f),
            ) {
              Row(
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth(),
              ) {
                Icon(
                  icon = Lucide.MessageSquare,
                  modifier = Modifier.size(20.dp),
                  tint = AppTheme.colors.textSubtle,
                )
                Spacer(Modifier.size(16.dp))
              }

              Text(
                "의견 보내기",
                style = AppTheme.typography.title,
              )
            }
          }

          CardSurface(
            modifier = Modifier.fillMaxWidth(),
          ) {
            Column {
              if (hasSubscription) {
                CardRow(
                  onClick = { uriHandler.openUri("https://typie.link/community") },
                ) {
                  AsyncImage(
                    model = Res.getUri("files/brands/discord.svg"),
                    contentDescription = null,
                    modifier = Modifier.size(20.dp),
                    colorFilter = ColorFilter.tint(AppTheme.colors.textSubtle),
                  )
                  Text(
                    "타이피 유저 커뮤니티",
                    modifier = Modifier.weight(1f),
                    style = AppTheme.typography.title,
                  )
                  Icon(
                    icon = Lucide.ExternalLink,
                    modifier = Modifier.size(16.dp),
                    tint = AppTheme.colors.textFaint,
                  )
                }

                CardDivider()
              }

              CardRow(
                onClick = { uriHandler.openUri("https://x.com/typieofficial") },
              ) {
                AsyncImage(
                  model = Res.getUri("files/brands/x.svg"),
                  contentDescription = null,
                  modifier = Modifier.size(18.dp),
                  colorFilter = ColorFilter.tint(AppTheme.colors.textSubtle),
                )
                Text(
                  "타이피 공식 X",
                  modifier = Modifier.weight(1f),
                  style = AppTheme.typography.title,
                )
                Icon(
                  icon = Lucide.ExternalLink,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textFaint,
                )
              }

              CardDivider()

              CardRow(
                onClick = { uriHandler.openUri("https://typie.co/changelog") },
              ) {
                Icon(
                  icon = Lucide.Newspaper,
                  modifier = Modifier.size(20.dp),
                  tint = AppTheme.colors.textSubtle,
                )
                Text(
                  "업데이트 노트",
                  modifier = Modifier.weight(1f),
                  style = AppTheme.typography.title,
                )
                Icon(
                  icon = Lucide.ExternalLink,
                  modifier = Modifier.size(16.dp),
                  tint = AppTheme.colors.textFaint,
                )
              }
            }
          }

          Column(
            modifier = Modifier.fillMaxWidth(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(12.dp),
          ) {
            Row(
              horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
              ThemeMode.entries.forEach { mode ->
                val isSelected = themeMode.value == mode

                Text(
                  text = mode.name,
                  style = AppTheme.typography.action,
                  color = if (isSelected) AppTheme.colors.textBrand else AppTheme.colors.textMuted,
                  modifier = Modifier
                    .clip(RoundedCornerShape(8.dp))
                    .clickable {
                      themeMode.value = mode
                      toast.show(ToastType.Success, "Theme mode changed to ${mode.name}")
                    }
                    .padding(horizontal = 8.dp, vertical = 4.dp),
                )
              }
            }

            Text(
              "로그아웃",
              style = AppTheme.typography.action,
              color = AppTheme.colors.textDanger,
              modifier = Modifier.clickable { scope.launch { authService.logout() } },
            )
          }

          Spacer(Modifier.height(140.dp))
        }
      }
    }
  }
}

private fun String.toActivityGridDateOrNull(): LocalDate? {
  return runCatching { LocalDate.parse(this) }.getOrNull()
    ?: toInstantOrNull()?.toLocalDateTime(TimeZone.currentSystemDefault())?.date
}
