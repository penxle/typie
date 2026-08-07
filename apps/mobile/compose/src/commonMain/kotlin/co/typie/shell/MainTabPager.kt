package co.typie.shell

import androidx.compose.foundation.MutatePriority
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.interaction.collectIsDraggedAsState
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.PagerDefaults
import androidx.compose.foundation.pager.PagerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.unit.Velocity
import co.typie.platform.LocalSoftwareKeyboardPresentationController
import co.typie.platform.SoftwareKeyboardPresentationEndpoint
import co.typie.platform.SoftwareKeyboardPresentationSession
import kotlin.math.abs
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.launch

internal enum class MainTabMotionSource {
  DirectDrag,
  Committed,
}

internal data class MainTabMotion(val origin: Tab, val target: Tab, val source: MainTabMotionSource)

@Stable
internal class MainTabState(initialTab: Tab, private val scope: CoroutineScope) {
  internal val pagerState = PagerState(currentPage = initialTab.ordinal) { Tab.entries.size }

  var settledTab by mutableStateOf(initialTab)
    private set

  var motion by mutableStateOf<MainTabMotion?>(null)
    private set

  val bodyPosition: Float
    get() =
      if (motion == null) {
        settledTab.ordinal.toFloat()
      } else {
        pagerState.currentPage + pagerState.currentPageOffsetFraction
      }

  private var selectionJob: Job? = null
  private var neutralizeJob: Job? = null
  private var selectionRequestId = 0L
  private var nextMotionSessionId = 0L
  private var activeMotionSessionId: Long? = null
  private var activeMotionObservedScroll = false
  private var returningToOrigin = false

  fun selectTab(tab: Tab, beforeTransition: suspend () -> Unit = {}) {
    if (motion?.source == MainTabMotionSource.DirectDrag) return

    selectionJob?.cancel()
    val requestId = ++selectionRequestId
    selectionJob = scope.launch {
      beforeTransition()
      if (requestId != selectionRequestId) return@launch

      val activeMotion = motion
      val origin = activeMotion?.origin ?: settledTab
      if (activeMotion == null && tab == origin) return@launch

      beginMotion(
        origin = origin,
        target = tab,
        source = MainTabMotionSource.Committed,
        observedScroll = pagerState.isScrollInProgress,
      )
      pagerState.animateScrollToPage(tab.ordinal)
    }
  }

  fun cancelToOrigin() {
    val activeMotion = motion ?: return
    val sessionId = activeMotionSessionId ?: return
    val interruptedSelection = selectionJob

    interruptedSelection?.cancel()
    selectionRequestId += 1
    returningToOrigin = true
    motion = activeMotion.copy(target = activeMotion.origin, source = MainTabMotionSource.Committed)
    selectionJob = scope.launch {
      interruptedSelection?.join()
      pagerState.scroll(MutatePriority.PreventUserInput) {
        with(pagerState) { updateCurrentPage(activeMotion.origin.ordinal) }
      }
      completeMotion(sessionId)
    }
  }

  internal fun reconcileRawPager(raw: RawMainTabPagerSnapshot): Tab? {
    val rawTarget = Tab.entries[raw.targetPage]

    if (raw.isDirectDrag) {
      if (motion == null) {
        if (raw.gestureAdmissionAllowed) {
          beginMotion(
            origin = settledTab,
            target = rawTarget,
            source = MainTabMotionSource.DirectDrag,
            observedScroll = raw.isScrollInProgress,
          )
        } else {
          requestSettledPage()
          return null
        }
      }

      val activeMotion = motion
      if (
        activeMotion?.source == MainTabMotionSource.DirectDrag && activeMotion.target != rawTarget
      ) {
        motion = activeMotion.copy(target = rawTarget)
      }
    } else {
      val activeMotion = motion
      if (activeMotion?.source == MainTabMotionSource.DirectDrag) {
        motion = activeMotion.copy(target = rawTarget, source = MainTabMotionSource.Committed)
      }
    }

    var activeMotion = motion
    if (activeMotion == null) {
      if (raw.isScrollInProgress && rawTarget != settledTab && raw.gestureAdmissionAllowed) {
        beginMotion(
          origin = settledTab,
          target = rawTarget,
          source = MainTabMotionSource.Committed,
          observedScroll = true,
        )
      } else if (raw.isDisplacedFrom(settledTab)) {
        requestSettledPage()
      }
      return null
    }

    val sessionId = activeMotionSessionId ?: return null
    if (raw.isScrollInProgress) activeMotionObservedScroll = true

    if (
      !returningToOrigin &&
        (raw.isScrollInProgress || activeMotionObservedScroll) &&
        activeMotion.target != rawTarget
    ) {
      motion = activeMotion.copy(target = rawTarget)
      activeMotion = motion ?: return null
    }

    if (
      !raw.isScrollInProgress &&
        activeMotion.source == MainTabMotionSource.Committed &&
        !returningToOrigin &&
        (activeMotionObservedScroll ||
          (activeMotion.target == activeMotion.origin && !raw.isDisplacedFrom(activeMotion.origin)))
    ) {
      return completeMotion(sessionId)
    }

    return null
  }

  private fun beginMotion(
    origin: Tab,
    target: Tab,
    source: MainTabMotionSource,
    observedScroll: Boolean,
  ) {
    neutralizeJob?.cancel()
    neutralizeJob = null
    activeMotionSessionId = ++nextMotionSessionId
    activeMotionObservedScroll = observedScroll
    returningToOrigin = false
    motion = MainTabMotion(origin = origin, target = target, source = source)
  }

  private fun completeMotion(sessionId: Long): Tab? {
    if (activeMotionSessionId != sessionId) return null

    val activeMotion = motion ?: return null
    val rawSettledTab = Tab.entries[pagerState.settledPage]
    if (returningToOrigin && rawSettledTab != activeMotion.origin) {
      requestSettledPage(activeMotion.origin)
      return null
    }

    settledTab = if (returningToOrigin) activeMotion.origin else rawSettledTab
    motion = null
    activeMotionSessionId = null
    activeMotionObservedScroll = false
    returningToOrigin = false
    return settledTab
  }

  private fun requestSettledPage(tab: Tab = settledTab) {
    pagerState.requestScrollToPage(tab.ordinal)
    if (!pagerState.isScrollInProgress || neutralizeJob?.isActive == true) return

    neutralizeJob = scope.launch {
      pagerState.scroll(MutatePriority.PreventUserInput) {
        pagerState.requestScrollToPage(tab.ordinal)
      }
    }
  }
}

@Composable
internal fun rememberMainTabState(initialTab: Tab = Tab.Home): MainTabState {
  val scope = rememberCoroutineScope()
  return remember { MainTabState(initialTab = initialTab, scope = scope) }
}

internal data class RawMainTabPagerSnapshot(
  val isDirectDrag: Boolean,
  val isScrollInProgress: Boolean,
  val currentPage: Int,
  val targetPage: Int,
  val currentPageOffsetFraction: Float,
  val gestureAdmissionAllowed: Boolean,
) {
  fun isDisplacedFrom(tab: Tab): Boolean =
    currentPage != tab.ordinal ||
      targetPage != tab.ordinal ||
      abs(currentPageOffsetFraction) > PagerPositionEpsilon
}

@Composable
internal fun MainTabPager(
  state: MainTabState,
  gestureAdmissionAllowed: Boolean,
  navigationLocked: Boolean = false,
  modifier: Modifier = Modifier,
  onSettledTab: (Tab) -> Unit = {},
  content: @Composable (Tab) -> Unit,
) {
  val pagerState = state.pagerState
  val focusManager = LocalFocusManager.current
  val softwareKeyboardPresentationController = LocalSoftwareKeyboardPresentationController.current
  val isDirectDrag by pagerState.interactionSource.collectIsDraggedAsState()
  val latestGestureAdmissionAllowed by rememberUpdatedState(gestureAdmissionAllowed)
  val latestOnSettledTab by rememberUpdatedState(onSettledTab)
  val userScrollEnabled = gestureAdmissionAllowed || state.motion != null
  val deferredUserScrollEnabled = rememberMainTabPagerUserScrollEnabled(userScrollEnabled)
  val defaultPageNestedScrollConnection =
    PagerDefaults.pageNestedScrollConnection(pagerState, Orientation.Horizontal)

  LaunchedEffect(state) {
    snapshotFlow {
        state.motion to
          RawMainTabPagerSnapshot(
            isDirectDrag = isDirectDrag,
            isScrollInProgress = pagerState.isScrollInProgress,
            currentPage = pagerState.currentPage,
            targetPage = pagerState.targetPage,
            currentPageOffsetFraction = pagerState.currentPageOffsetFraction,
            gestureAdmissionAllowed = latestGestureAdmissionAllowed,
          )
      }
      .collect { (_, raw) ->
        state.reconcileRawPager(raw)?.let { settledTab -> latestOnSettledTab(settledTab) }
      }
  }

  LaunchedEffect(state, softwareKeyboardPresentationController, focusManager) {
    var origin: Tab? = null
    var keyboardSession: SoftwareKeyboardPresentationSession? = null

    try {
      snapshotFlow { Triple(state.motion, state.bodyPosition, state.settledTab) }
        .collect { (motion, bodyPosition, settledTab) ->
          if (motion != null) {
            if (origin == null) {
              origin = motion.origin
              keyboardSession = softwareKeyboardPresentationController.acquire()
            }
            val activeOrigin = origin ?: return@collect
            keyboardSession?.updateHiddenProgress(
              abs(bodyPosition - activeOrigin.ordinal).coerceIn(0f, 1f)
            )
            return@collect
          }

          val activeOrigin = origin ?: return@collect
          val activeKeyboardSession = keyboardSession
          origin = null
          keyboardSession = null
          if (settledTab == activeOrigin) {
            activeKeyboardSession?.finish(SoftwareKeyboardPresentationEndpoint.Shown)
          } else {
            activeKeyboardSession?.finish(SoftwareKeyboardPresentationEndpoint.Hidden)
            focusManager.clearFocus(force = true)
          }
        }
    } finally {
      keyboardSession?.dispose()
    }
  }

  LaunchedEffect(navigationLocked, state.motion?.origin, state.motion != null) {
    if (navigationLocked && state.motion != null) state.cancelToOrigin()
  }

  HorizontalPager(
    state = pagerState,
    modifier = modifier,
    userScrollEnabled = deferredUserScrollEnabled,
    pageNestedScrollConnection =
      if (deferredUserScrollEnabled) {
        defaultPageNestedScrollConnection
      } else {
        DisabledMainTabPagerNestedScrollConnection
      },
    overscrollEffect = null,
  ) { page ->
    content(Tab.entries[page])
  }
}

@Composable
private fun rememberMainTabPagerUserScrollEnabled(requested: Boolean): Boolean {
  var enabledAfterFrame by remember { mutableStateOf(requested) }

  LaunchedEffect(requested) {
    if (!requested) {
      enabledAfterFrame = false
    } else if (!enabledAfterFrame) {
      // Re-enabling the pager while a root pop disposes movable route content can invalidate the
      // pager subcomposition in the same frame and crash Compose Runtime with a missing endGroup.
      // TODO: Remove this one-frame workaround after upgrading to a Compose Multiplatform release
      // where the root-pop flows no longer crash without it on iOS and JVM Desktop. The crash still
      // reproduces on Compose Multiplatform 1.12.0-beta03.
      withFrameNanos {}
      enabledAfterFrame = true
    }
  }

  return requested && enabledAfterFrame
}

private data object DisabledMainTabPagerNestedScrollConnection : NestedScrollConnection

internal fun mainTabActivationWeights(position: Float): Map<Tab, Float> =
  Tab.entries.associateWith { tab -> (1f - abs(position - tab.ordinal)).coerceIn(0f, 1f) }

internal fun mainTabChromeTab(settledTab: Tab, motion: MainTabMotion?): Tab =
  when (motion?.source) {
    MainTabMotionSource.DirectDrag -> motion.origin
    MainTabMotionSource.Committed -> motion.target
    null -> settledTab
  }

internal fun canAdmitMainTabPagerGesture(
  navigatorCanPop: Boolean,
  navigatorIsTransitioning: Boolean,
  motion: MainTabMotion?,
  drawerAtRest: Boolean,
): Boolean = !navigatorCanPop && !navigatorIsTransitioning && motion == null && drawerAtRest

internal fun mainTabPillHandoffProgress(
  position: Float,
  originPosition: Float,
  targetPosition: Float,
): Float {
  val distance = targetPosition - originPosition
  if (distance == 0f) return 1f
  return ((position - originPosition) / distance).coerceIn(0f, 1f)
}

internal fun Modifier.mainTabPagerChildHorizontalGesture(): Modifier =
  nestedScroll(MainTabPagerChildHorizontalHandoffBoundary)

private data object MainTabPagerChildHorizontalHandoffBoundary : NestedScrollConnection {
  override fun onPostScroll(
    consumed: Offset,
    available: Offset,
    source: NestedScrollSource,
  ): Offset =
    if (source == NestedScrollSource.UserInput) Offset(x = available.x, y = 0f) else Offset.Zero

  override suspend fun onPostFling(consumed: Velocity, available: Velocity): Velocity =
    Velocity(x = available.x, y = 0f)
}

private const val PagerPositionEpsilon = 0.001f
