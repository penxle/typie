package co.typie.shell

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalInteractionSource
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.touchShield
import co.typie.ext.toPx
import co.typie.icons.Lucide
import co.typie.navigation.NavigationScaffold
import co.typie.navigation.NavigationStack
import co.typie.navigation.Navigator
import co.typie.overlay.Toast
import co.typie.route.Route
import co.typie.route.toastBottomInset
import co.typie.shell.marketing_consent.MarketingConsentGate
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import org.koin.compose.koinInject

private enum class Tab(val route: Route) {
  Home(Route.Home), Space(Route.Space), Notes(Route.Notes), More(Route.More),
}

@Stable
class BottomBarState {
  var visible by mutableStateOf(true)
}

val LocalBottomBarState =
  compositionLocalOf<BottomBarState> { error("LocalBottomBarState not provided") }

private const val FAB_SIZE = 60
private const val FAB_GAP = 8

@Composable
fun MainShell(content: @Composable (Route) -> Unit) {
  var currentTab by remember { mutableStateOf(Tab.entries.first()) }
  val navigators = remember {
    Tab.entries.associateWith { Navigator(it.route) }
  }
  val activeNavigator = navigators[currentTab]!!
  val bottomBarState = remember { BottomBarState() }
  val showBottomBar = bottomBarState.visible && (activeNavigator.stack.size == 1 ||
    (activeNavigator.stack.size == 2 && activeNavigator.popRequested))

  val topBarState = remember { TopBarState() }

  val density = LocalDensity.current
  val toast = koinInject<Toast>()
  LaunchedEffect(activeNavigator.current) {
    toast.bottomInset = activeNavigator.current.toastBottomInset
  }

  DisposableEffect(Unit) {
    onDispose {
      navigators.values.forEach { it.clear() }
    }
  }

  NavigationScaffold(
    navigator = activeNavigator,
    topBarState = topBarState,
    overlay = {
      AnimatedVisibility(
        visible = showBottomBar,
        modifier = Modifier.align(Alignment.BottomCenter),
        enter = slideInVertically(
          initialOffsetY = { 64.dp.toPx(density).toInt() },
          animationSpec = tween(300, easing = EaseOutCubic),
        ) + fadeIn(animationSpec = tween(200)),
        exit = slideOutVertically(
          targetOffsetY = { 64.dp.toPx(density).toInt() },
          animationSpec = tween(300, easing = EaseOutCubic),
        ) + fadeOut(animationSpec = tween(200)),
      ) {
        BottomBar(
          currentTab = currentTab,
          onSelectTab = { currentTab = it },
        )
      }
    },
  ) {
    CompositionLocalProvider(LocalBottomBarState provides bottomBarState) {
      SiteUpdateStreamEffect()

      Crossfade(
        targetState = currentTab,
        modifier = Modifier.fillMaxSize(),
        animationSpec = tween(200),
      ) { tab ->
        NavigationStack(
          navigator = navigators[tab]!!,
          topBarState = topBarState,
          content = content,
        )
      }
    }
  }

  MarketingConsentGate()
}

@Composable
private fun BottomBar(currentTab: Tab, onSelectTab: (Tab) -> Unit, modifier: Modifier = Modifier) {
  val colors = AppTheme.colors
  val pillInteractionSource = remember { MutableInteractionSource() }
  val pillScale = remember { Animatable(1f) }
  val isPillPressed by pillInteractionSource.collectIsPressedAsState()

  LaunchedEffect(isPillPressed) {
    if (isPillPressed) {
      pillScale.animateTo(1.03f, tween(150, easing = EaseOutCubic))
    } else {
      pillScale.animateTo(1f, spring(dampingRatio = 0.6f, stiffness = 300f))
    }
  }

  Box(
    modifier
      .fillMaxWidth()
      .navigationBarsPadding()
      .padding(horizontal = 24.dp)
      .padding(bottom = 12.dp),
    contentAlignment = Alignment.Center,
  ) {
    Row(
      Modifier.widthIn(max = 488.dp).fillMaxWidth(),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      // Pill
      Box(
        Modifier.weight(1f).touchShield().graphicsLayer {
          scaleX = pillScale.value
          scaleY = pillScale.value
        },
      ) {
        CompositionLocalProvider(LocalInteractionSource provides pillInteractionSource) {
          Row(
            Modifier.fillMaxWidth()
              .height(60.dp)
              .dropShadow(CircleShape) {
                color = colors.shadowAmbient
                radius = 8f
              }
              .dropShadow(CircleShape) {
                color = colors.shadow
                offset = Offset(0f, 4f)
                radius = 12f
              }
              .dropShadow(CircleShape) {
                color = colors.shadow
                offset = Offset(0f, 12f)
                radius = 32f
              }
              .border(1.dp, AppTheme.colors.borderDefault, CircleShape)
              .background(AppTheme.colors.surfaceRaised, CircleShape),
          ) {
            Tab.entries.forEach { tab ->
              val selected = tab == currentTab
              val bgColor by animateColorAsState(
                targetValue = if (selected) AppTheme.colors.surfaceTinted else AppTheme.colors.surfaceBase.copy(
                  alpha = 0f
                ),
                animationSpec = tween(200),
              )

              Box(
                modifier = Modifier
                  .weight(1f)
                  .fillMaxHeight()
                  .padding(4.dp)
                  .background(bgColor, CircleShape)
                  .clickable { onSelectTab(tab) },
                contentAlignment = Alignment.Center,
              ) {
                Icon(
                  icon = when (tab) {
                    Tab.Home -> Lucide.House
                    Tab.Space -> Lucide.FolderOpen
                    Tab.Notes -> Lucide.StickyNote
                    Tab.More -> Lucide.Ellipsis
                  },
                  tint = AppTheme.colors.textSecondary,
                )
              }
            }
          }
        }
      }

      Spacer(Modifier.width(FAB_GAP.dp))

      // FAB
      Fab()
    }
  }
}

@Composable
private fun Fab(modifier: Modifier = Modifier) {
  val colors = AppTheme.colors
  val fabInteractionSource = remember { MutableInteractionSource() }
  val fabScale = remember { Animatable(1f) }
  val isFabPressed by fabInteractionSource.collectIsPressedAsState()

  LaunchedEffect(isFabPressed) {
    if (isFabPressed) {
      fabScale.animateTo(1.05f, tween(150, easing = EaseOutCubic))
    } else {
      fabScale.animateTo(1f, spring(dampingRatio = 0.6f, stiffness = 300f))
    }
  }

  CompositionLocalProvider(LocalInteractionSource provides fabInteractionSource) {
    Box(
      modifier.size(FAB_SIZE.dp)
        .graphicsLayer {
          scaleX = fabScale.value
          scaleY = fabScale.value
        }
        .dropShadow(CircleShape) {
          color = colors.shadowAmbient
          radius = 8f
        }
        .dropShadow(CircleShape) {
          color = colors.shadow
          offset = Offset(0f, 4f)
          radius = 12f
        }
        .dropShadow(CircleShape) {
          color = colors.shadow
          offset = Offset(0f, 12f)
          radius = 32f
        }
        .background(AppTheme.colors.surfaceRaised, CircleShape)
        .border(1.dp, AppTheme.colors.borderDefault, CircleShape).clickable { /* TODO */ },
      contentAlignment = Alignment.Center,
    ) {
      Icon(
        icon = Lucide.SquarePen,
        tint = AppTheme.colors.textSecondary,
      )
    }
  }
}
