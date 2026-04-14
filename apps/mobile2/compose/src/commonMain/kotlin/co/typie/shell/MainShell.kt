package co.typie.shell

import androidx.compose.animation.Crossfade
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalInteractionSource
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.touchShield
import co.typie.graphql.Apollo
import co.typie.graphql.MainShell_SiteUpdateStream_Subscription
import co.typie.icons.Lucide
import co.typie.navigation.NavigationScaffold
import co.typie.navigation.NavigationStack
import co.typie.navigation.Navigator
import co.typie.route.Route
import co.typie.route.toastBottomInset
import co.typie.shell.marketing_consent.MarketingConsentGate
import co.typie.storage.Preference
import co.typie.ui.component.bottombar.ACTION_BUTTON_TOTAL_WIDTH
import co.typie.ui.component.bottombar.BottomBarActionButton
import co.typie.ui.component.bottombar.BottomBarDefaults
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.flow.collect

@Composable
fun MainShell(content: @Composable (Route) -> Unit) {
  var currentTab by remember { mutableStateOf(Tab.entries.first()) }
  val navigators = remember { Tab.entries.associateWith { Navigator(it.route) } }
  val activeNavigator = navigators[currentTab]!!

  val topBarState = remember { TopBarState() }
  val bottomBarState = remember { BottomBarState() }

  val siteId = Preference.siteId

  val toast = LocalToast.current
  LaunchedEffect(activeNavigator.current) {
    toast.bottomInset = activeNavigator.current.toastBottomInset
  }

  DisposableEffect(Unit) { onDispose { navigators.values.forEach { it.clear() } } }

  LaunchedEffect(siteId) {
    if (siteId.isNullOrBlank()) {
      return@LaunchedEffect
    }

    Apollo.subscription(MainShell_SiteUpdateStream_Subscription(siteId = siteId))
      .retryOnError(true)
      .toFlow()
      .collect()
  }

  CompositionLocalProvider(
    LocalTabState provides TabState(currentTab = currentTab, onSelectTab = { currentTab = it })
  ) {
    NavigationScaffold(
      navigator = activeNavigator,
      topBarState = topBarState,
      bottomBarState = bottomBarState,
    ) {
      Crossfade(
        targetState = currentTab,
        modifier = Modifier.fillMaxSize(),
        animationSpec = tween(200),
      ) { tab ->
        NavigationStack(
          navigator = navigators[tab]!!,
          topBarState = topBarState,
          bottomBarState = bottomBarState,
          content = content,
        )
      }
    }
  }

  MarketingConsentGate()
}

@Composable
fun MainBottomBarPill() {
  val tabState = LocalTabState.current
  Box(Modifier.fillMaxSize(), contentAlignment = Alignment.BottomCenter) {
    BottomBarPill(currentTab = tabState.currentTab, onSelectTab = tabState.onSelectTab)
  }
}

@Composable
private fun BottomBarPill(
  currentTab: Tab,
  onSelectTab: (Tab) -> Unit,
  modifier: Modifier = Modifier,
) {
  val colors = AppTheme.colors
  val pillInteractionSource = remember { MutableInteractionSource() }
  val pillScale = remember { Animatable(1f) }
  val isPillPressed by pillInteractionSource.collectIsPressedAsState()

  LaunchedEffect(isPillPressed) {
    if (isPillPressed) {
      pillScale.animateTo(1.01f, tween(150, easing = EaseOutCubic))
    } else {
      pillScale.animateTo(1f, spring(dampingRatio = 0.6f, stiffness = 300f))
    }
  }

  Box(
    modifier
      .fillMaxWidth()
      .navigationBarsPadding()
      .padding(horizontal = 24.dp)
      .padding(bottom = BottomBarDefaults.BottomPadding),
    contentAlignment = Alignment.Center,
  ) {
    Row(
      Modifier.widthIn(max = 488.dp).fillMaxWidth(),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Box(
        Modifier.weight(1f).touchShield().graphicsLayer {
          scaleX = pillScale.value
          scaleY = pillScale.value
        }
      ) {
        CompositionLocalProvider(LocalInteractionSource provides pillInteractionSource) {
          Row(
            Modifier.fillMaxWidth()
              .height(BottomBarDefaults.PillHeight)
              .dropShadow(AppShapes.circle) {
                color = colors.shadowAmbient
                radius = 3f
              }
              .dropShadow(AppShapes.circle) {
                color = colors.shadow
                offset = Offset(0f, 4f)
                radius = 16f
              }
              .background(AppTheme.colors.surfaceRaised, AppShapes.circle)
              .border(1.dp, AppTheme.colors.borderDefault.copy(alpha = 0.5f), AppShapes.circle)
          ) {
            Tab.entries.forEach { tab ->
              val selected = tab == currentTab
              val bgColor by
                animateColorAsState(
                  targetValue =
                    if (selected) AppTheme.colors.surfaceTinted
                    else AppTheme.colors.surfaceBase.copy(alpha = 0f),
                  animationSpec = tween(200),
                )

              Box(
                modifier =
                  Modifier.weight(1f)
                    .fillMaxHeight()
                    .padding(3.dp)
                    .background(bgColor, AppShapes.circle)
                    .clickable { onSelectTab(tab) },
                contentAlignment = Alignment.Center,
              ) {
                Icon(
                  icon =
                    when (tab) {
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

      Spacer(Modifier.width(ACTION_BUTTON_TOTAL_WIDTH.dp))
    }
  }
}

@Composable
fun MainBottomBarActionButton(onClick: suspend () -> Unit = {}) {
  BottomBarActionButton(icon = Lucide.SquarePen, onClick = onClick)
}
