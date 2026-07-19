package co.typie.screen.system.onboarding

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.painter.Painter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.generated.resources.Res
import co.typie.navigation.Nav
import co.typie.storage.Preference
import co.typie.ui.component.Button
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.resolveIsDarkTheme
import io.github.alexzhirkevich.compottie.Compottie
import io.github.alexzhirkevich.compottie.LottieCompositionSpec
import io.github.alexzhirkevich.compottie.rememberLottieComposition
import io.github.alexzhirkevich.compottie.rememberLottiePainter
import kotlinx.coroutines.launch

private data class OnboardingPage(
  val asset: String,
  val loop: Boolean,
  val heroFraction: Float,
  val title: String,
  val subtitle: String,
)

private val pages =
  listOf(
    OnboardingPage(
      asset = "logo",
      loop = false,
      heroFraction = 0.55f,
      title = "타이피에 오신 걸 환영해요",
      subtitle = "떠오른 순간을 놓치지 않도록,\n언제 어디서나 편안하게 글을 이어 써보세요.",
    ),
    OnboardingPage(
      asset = "writing",
      loop = true,
      heroFraction = 1f,
      title = "글을 쓰는 모든 순간을 한곳에서",
      subtitle = "작품과 설정을 스페이스로 정리하고,\n나에게 맞는 환경에서 쓰고 공유해 보세요.",
    ),
    OnboardingPage(
      asset = "features",
      loop = true,
      heroFraction = 1f,
      title = "14일 무료 체험이 시작됐어요",
      subtitle = "타이피의 모든 기능을 이용할 수 있어요.\n지금 바로 첫 글을 시작해보세요.",
    ),
  )

@Composable
fun OnboardingPreviewScreen() {
  val nav = Nav.current
  val scope = rememberCoroutineScope()
  OnboardingScreen(onComplete = { scope.launch { nav.pop() } })
}

@Composable
fun OnboardingScreen(onComplete: () -> Unit) {
  val nav = Nav.current
  val pagerState = rememberPagerState { pages.size }
  val isLast = pagerState.currentPage == pages.lastIndex

  ProvideTopBar(
    leading =
      if (nav.canPop) {
        { TopBarBackButton() }
      } else {
        null
      },
    trailing =
      if (!isLast) {
        {
          Text(
            text = "건너뛰기",
            style = AppTheme.typography.action,
            color = AppTheme.colors.textMuted,
            modifier = Modifier.clickable { onComplete() },
          )
        }
      } else {
        null
      },
  )

  Screen(background = AppTheme.colors.surfaceDefault) { contentPadding ->
    Column(modifier = Modifier.fillMaxSize().padding(contentPadding)) {
      HorizontalPager(
        state = pagerState,
        beyondViewportPageCount = pages.size - 1,
        modifier = Modifier.weight(1f),
      ) { index ->
        val page = pages[index]
        Column(modifier = Modifier.fillMaxSize()) {
          Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
            Image(
              painter = onboardingLottiePainter(page, playing = pagerState.currentPage == index),
              contentDescription = null,
              contentScale = ContentScale.Fit,
              modifier =
                Modifier.align(Alignment.Center)
                  .fillMaxWidth(page.heroFraction)
                  .padding(horizontal = 32.dp)
                  .aspectRatio(1f),
            )
          }

          Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
          ) {
            Text(
              text = page.title,
              style = AppTheme.typography.heading,
              color = AppTheme.colors.textDefault,
              textAlign = TextAlign.Center,
              modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(12.dp))
            Text(
              text = page.subtitle,
              style = AppTheme.typography.body,
              color = AppTheme.colors.textMuted,
              textAlign = TextAlign.Center,
              modifier = Modifier.fillMaxWidth(),
              minLines = 2,
            )
          }
        }
      }

      Spacer(Modifier.height(12.dp))

      Row(modifier = Modifier.align(Alignment.CenterHorizontally)) {
        repeat(pages.size) { index ->
          val active = pagerState.currentPage == index
          val dotWidth by
            animateDpAsState(
              targetValue = if (active) 24.dp else 8.dp,
              animationSpec = spring(dampingRatio = 0.72f, stiffness = Spring.StiffnessMediumLow),
            )
          Box(
            modifier =
              Modifier.clickable {
                  pagerState.animateScrollToPage(
                    page = index,
                    animationSpec = tween(durationMillis = 450, easing = FastOutSlowInEasing),
                  )
                }
                .padding(horizontal = 4.dp, vertical = 12.dp),
            contentAlignment = Alignment.Center,
          ) {
            Box(
              modifier =
                Modifier.height(8.dp)
                  .width(dotWidth)
                  .clip(CircleShape)
                  .background(
                    if (active) AppTheme.colors.textDefault
                    else AppTheme.colors.textMuted.copy(alpha = 0.3f)
                  )
            )
          }
        }
      }

      Spacer(Modifier.height(8.dp))

      Button(
        text = if (isLast) "첫 글 시작하기" else "다음",
        onClick = {
          if (isLast) {
            onComplete()
          } else {
            pagerState.animateScrollToPage(
              page = pagerState.currentPage + 1,
              animationSpec = tween(durationMillis = 450, easing = FastOutSlowInEasing),
            )
          }
        },
        modifier = Modifier.fillMaxWidth(),
      )

      Spacer(Modifier.height(16.dp))
    }
  }
}

@Composable
private fun onboardingLottiePainter(page: OnboardingPage, playing: Boolean): Painter {
  val dark =
    resolveIsDarkTheme(themeMode = Preference.themeMode, systemIsDark = isSystemInDarkTheme())
  val variant = if (dark) "dark" else "light"
  val composition by
    rememberLottieComposition(page.asset, variant) {
      LottieCompositionSpec.JsonString(
        Res.readBytes("files/lottie/onboarding_${page.asset}_$variant.json").decodeToString()
      )
    }
  return rememberLottiePainter(
    composition = composition,
    iterations = if (page.loop) Compottie.IterateForever else 1,
    isPlaying = playing,
    restartOnPlay = false,
  )
}
