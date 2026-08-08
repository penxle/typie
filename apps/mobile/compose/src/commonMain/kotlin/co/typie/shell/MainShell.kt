package co.typie.shell

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import co.typie.domain.note.NoteSync
import co.typie.domain.note.NoteUpdate
import co.typie.domain.subscription.PlanChangeNoticeHost
import co.typie.domain.subscription.SubscriptionGateHost
import co.typie.ext.pointerIgnore
import co.typie.graphql.Apollo
import co.typie.graphql.MainShell_SiteUpdateStream_Subscription
import co.typie.graphql.Note_UpdateStream_Subscription
import co.typie.navigation.Nav
import co.typie.navigation.NavigationScaffold
import co.typie.navigation.NavigationStack
import co.typie.navigation.PlatformBackHandler
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.drawer.Drawer
import co.typie.ui.component.drawer.DrawerAnchor
import co.typie.ui.component.drawer.LocalDrawer
import co.typie.ui.component.topbar.TopBarState
import kotlin.math.abs
import kotlinx.coroutines.flow.collect

@Composable
fun MainShell(content: @Composable (Route) -> Unit) {
  val navState =
    rememberSaveable(saver = MainNavigationStateSaver) { MainNavigationState.initial() }
  val navigators = navState.navigators
  val mainTabState = rememberMainTabState(initialTab = navState.currentTab)
  val settledTab = mainTabState.settledTab
  val motion = mainTabState.motion
  val chromeTab = mainTabChromeTab(settledTab = settledTab, motion = motion)
  val chromeNavigator = navigators[chromeTab]!!
  val settledNavigator = navigators[settledTab]!!
  val motionOwnerNavigator = navigators[motion?.origin ?: settledTab]!!

  val topBarState = remember { TopBarState() }
  val bottomBarState = remember { BottomBarState() }
  val backgroundTopBarStates = remember { Tab.entries.associateWith { TopBarState() } }
  val backgroundBottomBarStates = remember { Tab.entries.associateWith { BottomBarState() } }
  val drawer = remember { Drawer() }

  val drawerAtRest by
    remember(drawer) {
      derivedStateOf {
        val drawerOffset = drawer.state.offset
        val drawerClosedOffset = drawer.state.anchors.positionOf(DrawerAnchor.Closed)
        !drawerOffset.isNaN() &&
          abs(drawerOffset - drawerClosedOffset) < 0.5f &&
          drawer.state.currentValue == DrawerAnchor.Closed &&
          drawer.state.targetValue == DrawerAnchor.Closed &&
          !drawer.state.isAnimationRunning &&
          !drawer.isProgrammaticAnimating
      }
    }
  val mayAdmitNewPagerGesture =
    canAdmitMainTabPagerGesture(
      navigatorCanPop = settledNavigator.canPop,
      navigatorIsTransitioning = settledNavigator.isTransitioning,
      motion = motion,
      drawerAtRest = drawerAtRest,
    )
  // A stable nested route blocks pager gestures, not explicit bottom-bar selection.
  val navigationLocksActiveMotion =
    motionOwnerNavigator.isTransitioning ||
      (motion?.source == MainTabMotionSource.DirectDrag && motionOwnerNavigator.canPop)
  val drawerSwipeEnabled =
    settledTab == Tab.Home &&
      chromeTab == Tab.Home &&
      !settledNavigator.canPop &&
      !settledNavigator.isTransitioning
  val drawerSwipeModifier =
    mainDrawerSwipeToOpenModifier(
      drawer = drawer,
      enabled = drawerSwipeEnabled,
      canStart = { drawerAtRest },
    )

  val latestDrawerAtRest by rememberUpdatedState(drawerAtRest)
  val selectTab =
    remember(mainTabState, drawer) {
      { tab: Tab -> mainTabState.selectTab(tab) { if (!latestDrawerAtRest) drawer.close() } }
    }

  val siteId = Preference.siteId

  DisposableEffect(Unit) { onDispose { navigators.values.forEach { it.clear() } } }

  LaunchedEffect(chromeTab) { if (chromeTab != Tab.Home && !drawerAtRest) drawer.close() }

  LaunchedEffect(siteId) {
    if (siteId == null) {
      return@LaunchedEffect
    }

    Apollo.subscription(MainShell_SiteUpdateStream_Subscription(siteId = siteId))
      .retryOnError(true)
      .toFlow()
      .collect()
  }

  LaunchedEffect(siteId) {
    if (siteId == null) {
      return@LaunchedEffect
    }

    Apollo.subscription(
        Note_UpdateStream_Subscription(siteId = siteId, clientId = NoteSync.clientId)
      )
      .retryOnError(true)
      .toFlow()
      .collect { response ->
        val payload = response.data?.noteUpdateStream ?: return@collect
        NoteSync.publish(NoteUpdate(kind = payload.kind, noteId = payload.noteId, siteId = siteId))
      }
  }

  CompositionLocalProvider(
    LocalTabState provides
      TabState(
        currentTab = chromeTab,
        bodyPosition = mainTabState.bodyPosition,
        isBodyMoving = motion != null,
        onSelectTab = selectTab,
      ),
    LocalDrawer provides drawer,
    Nav provides chromeNavigator,
  ) {
    Box(Modifier.fillMaxSize()) {
      NavigationScaffold(
        navigator = chromeNavigator,
        topBarState = topBarState,
        bottomBarState = bottomBarState,
        modifier = drawerSwipeModifier,
      ) {
        MainTabPager(
          state = mainTabState,
          modifier = Modifier.fillMaxSize(),
          gestureAdmissionAllowed = mayAdmitNewPagerGesture,
          navigationLocked = navigationLocksActiveMotion,
          onSettledTab = { tab -> navState.currentTab = tab },
        ) { tab ->
          val foregroundInteractive = tab == settledTab && motion == null && drawerAtRest
          val foregroundFocusEnabled = tab == mainTabState.focusEnabledTab && drawerAtRest
          NavigationStack(
            navigator = navigators[tab]!!,
            topBarState =
              if (tab == chromeTab) topBarState else backgroundTopBarStates.getValue(tab),
            bottomBarState =
              if (tab == chromeTab) bottomBarState else backgroundBottomBarStates.getValue(tab),
            foregroundInteractive = foregroundInteractive,
            foregroundFocusEnabled = foregroundFocusEnabled,
            content = content,
          )
        }
      }
      if (motion != null) {
        Box(Modifier.fillMaxSize().pointerIgnore())
      }
      MainDrawerOverlay(drawer)
      SubscriptionGateHost()
      PlanChangeNoticeHost()
    }
  }

  PlatformBackHandler(enabled = motion != null) { mainTabState.cancelToOrigin() }
}

enum class Tab(val route: Route) {
  Home(Route.Home),
  Space(Route.Space),
  Notes(Route.Notes),
}

class TabState(
  val currentTab: Tab,
  val bodyPosition: Float,
  val isBodyMoving: Boolean,
  val onSelectTab: (Tab) -> Unit,
)

val LocalTabState = compositionLocalOf<TabState> { error("LocalTabState not provided") }
