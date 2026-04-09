package co.typie.shell

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.IntrinsicSize
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.LocalInteractionSource
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pointerIgnore
import co.typie.ext.safeDrawing
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
import co.typie.ui.component.Text
import co.typie.ui.component.popover.AnchorPointerState
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.icon.Icon
import co.typie.ui.shape.SquircleShape
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
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
private const val FAB_MENU_GAP = 10
private const val FAB_SELECTION_ARM_DELAY_MS = 150L
private const val FAB_SAME_PRESS_SELECTION_DISTANCE_DP = 9

@Composable
fun MainShell(content: @Composable (Route) -> Unit) {
  var currentTab by remember { mutableStateOf(Tab.entries.first()) }
  val navigators = remember {
    Tab.entries.associateWith { Navigator(it.route) }
  }
  val activeNavigator = navigators[currentTab]!!
  val bottomBarState = remember { BottomBarState() }
  val showBottomBar = bottomBarState.visible && (
    activeNavigator.stack.size == 1 ||
      activeNavigator.current is Route.Folder ||
      (activeNavigator.stack.size == 2 && activeNavigator.popRequested)
    )
  val fabConfig = when {
    activeNavigator.current is Route.Folder -> spaceFabConfig()
    currentTab == Tab.Space && activeNavigator.current is Route.Space -> spaceFabConfig()
    else -> null
  }

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
      BottomBarOverlay(
        currentTab = currentTab,
        navVisible = showBottomBar,
        fabConfig = fabConfig,
        onSelectTab = { currentTab = it },
      )
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
private fun BoxScope.BottomBarOverlay(
  currentTab: Tab,
  navVisible: Boolean,
  fabConfig: FabConfig?,
  onSelectTab: (Tab) -> Unit,
  modifier: Modifier = Modifier,
) {
  BottomBarPill(
    currentTab = currentTab,
    navVisible = navVisible,
    onSelectTab = onSelectTab,
    modifier = modifier,
  )
  ShellFab(
    config = fabConfig,
    navVisible = navVisible,
  )
}

@Composable
private fun BoxScope.BottomBarPill(
  currentTab: Tab,
  navVisible: Boolean,
  onSelectTab: (Tab) -> Unit,
  modifier: Modifier = Modifier,
) {
  val colors = AppTheme.colors
  val pillInteractionSource = remember { MutableInteractionSource() }
  val pillScale = remember { Animatable(1f) }
  val isPillPressed by pillInteractionSource.collectIsPressedAsState()
  val density = LocalDensity.current
  val alpha by animateFloatAsState(targetValue = if (navVisible) 1f else 0f, animationSpec = tween(200))
  val translationY by animateFloatAsState(targetValue = if (navVisible) 0f else 120.dp.toPx(density), animationSpec = tween(300, easing = EaseOutCubic))

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
      .align(Alignment.BottomCenter)
      .navigationBarsPadding()
      .padding(horizontal = 24.dp)
      .padding(bottom = 12.dp)
      .graphicsLayer {
        this.alpha = alpha
        this.translationY = translationY
      }
      .then(if (navVisible) Modifier else Modifier.pointerIgnore()),
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

      Spacer(Modifier.width((FAB_SIZE + FAB_GAP).dp))
    }
  }
}

@Composable
private fun BoxScope.ShellFab(
  config: FabConfig?,
  navVisible: Boolean,
  modifier: Modifier = Modifier,
) {
  val colors = AppTheme.colors
  val density = LocalDensity.current
  val gestureScope = rememberCoroutineScope()
  val fabInteractionSource = remember { MutableInteractionSource() }
  val fabScale = remember { Animatable(1f) }
  val isFabPressed by fabInteractionSource.collectIsPressedAsState()
  val safeBottomPadding = WindowInsets.safeDrawing.asPaddingValues().calculateBottomPadding()
  val hasMenu = config?.menuItems?.isNotEmpty() == true
  val samePressSelectionDistance = FAB_SAME_PRESS_SELECTION_DISTANCE_DP.dp.toPx(density)
  var isMenuOpen by remember(config) { mutableStateOf(false) }
  var isMenuPressed by remember { mutableStateOf(false) }
  var buttonWindowTopLeft by remember { mutableStateOf(Offset.Zero) }
  var menuPointerState by remember { mutableStateOf<AnchorPointerState?>(null) }
  var trackedPointerOrigin by remember { mutableStateOf<Offset?>(null) }
  var trackedPointerPosition by remember { mutableStateOf<Offset?>(null) }
  var isTrackedPointerArmed by remember { mutableStateOf(false) }
  var isTrackedPointerHoldComplete by remember { mutableStateOf(false) }
  var selectionArmJob by remember { mutableStateOf<Job?>(null) }
  val buttonAlpha by animateFloatAsState(targetValue = if (navVisible) 1f else 0f, animationSpec = tween(200))
  val buttonTranslationY by animateFloatAsState(targetValue = if (navVisible) 0f else 120.dp.toPx(density), animationSpec = tween(300, easing = EaseOutCubic))

  fun updateTrackedPointerArmState(windowPosition: Offset) {
    if (isTrackedPointerArmed || !isTrackedPointerHoldComplete) {
      return
    }

    val origin = trackedPointerOrigin ?: return
    if ((windowPosition - origin).getDistance() <= samePressSelectionDistance) {
      return
    }

    isTrackedPointerArmed = true
    menuPointerState = AnchorPointerState(
      position = windowPosition,
      isSelectionArmed = true,
      isUp = false,
    )
  }

  fun resetTrackedPointer() {
    selectionArmJob?.cancel()
    selectionArmJob = null
    menuPointerState = null
    trackedPointerOrigin = null
    trackedPointerPosition = null
    isTrackedPointerArmed = false
    isTrackedPointerHoldComplete = false
    isMenuPressed = false
  }

  LaunchedEffect(config) {
    isMenuOpen = false
    resetTrackedPointer()
  }

  LaunchedEffect(navVisible) {
    if (!navVisible) {
      isMenuOpen = false
      resetTrackedPointer()
    }
  }

  LaunchedEffect(if (hasMenu) isMenuPressed else isFabPressed) {
    if (if (hasMenu) isMenuPressed else isFabPressed) {
      fabScale.animateTo(1.05f, tween(150, easing = EaseOutCubic))
    } else {
      fabScale.animateTo(1f, spring(dampingRatio = 0.6f, stiffness = 300f))
    }
  }

  BoxWithConstraints(
    modifier = modifier
      .fillMaxSize()
      .align(Alignment.BottomCenter),
  ) {
    val shellWidth = (maxWidth - 48.dp).coerceAtMost(488.dp).coerceAtLeast(0.dp)
    val shellHorizontalInset = (maxWidth - shellWidth) / 2

    if (isMenuOpen && hasMenu) {
      Box(
        modifier = Modifier
          .fillMaxSize()
          .pointerInput(Unit) {
            detectTapGestures {
              resetTrackedPointer()
              isMenuOpen = false
            }
          },
      )
    }

    if (hasMenu && config != null) {
      AnimatedVisibility(
        visible = isMenuOpen,
        modifier = Modifier
          .align(Alignment.BottomEnd)
          .padding(
            end = shellHorizontalInset,
            bottom = safeBottomPadding + 12.dp + FAB_SIZE.dp + FAB_MENU_GAP.dp,
          ),
        enter = fadeIn(animationSpec = tween(280)) + slideInVertically(
          animationSpec = tween(280, easing = EaseOutCubic),
          initialOffsetY = { (it * 0.12f).toInt() },
        ),
        exit = fadeOut(animationSpec = tween(180)) + slideOutVertically(
          animationSpec = tween(180, easing = EaseOutCubic),
          targetOffsetY = { (it * 0.12f).toInt() },
        ),
      ) {
        Box(
          modifier = Modifier
            .dropShadow(SquircleShape(22.dp)) {
              color = colors.shadow.copy(alpha = 0.08f)
              radius = 8f
            }
            .background(AppTheme.colors.surfaceRaised, SquircleShape(22.dp))
            .border(1.dp, AppTheme.colors.borderDefault, SquircleShape(22.dp)),
        ) {
          Column(
            modifier = Modifier
              .width(IntrinsicSize.Max)
              .padding(6.dp),
          ) {
            PopoverList(
              items = config.menuItems.map { item ->
                PopoverListItem(
                  content = {
                    FabMenuItemRow(
                      item = item,
                      modifier = Modifier
                        .height(42.dp)
                        .padding(horizontal = 16.dp),
                    )
                  },
                  onSelected = {
                    resetTrackedPointer()
                    isMenuOpen = false
                    item.onClick()
                  },
                )
              },
              pointerState = menuPointerState,
              inputEnabled = isMenuOpen,
              armDelayMs = FAB_SELECTION_ARM_DELAY_MS,
            )
          }
        }
      }
    }

    CompositionLocalProvider(LocalInteractionSource provides fabInteractionSource) {
      Box(
        modifier = Modifier
          .align(Alignment.BottomEnd)
          .padding(end = shellHorizontalInset, bottom = safeBottomPadding + 12.dp)
          .size(FAB_SIZE.dp)
          .onGloballyPositioned { coordinates ->
            buttonWindowTopLeft = coordinates.positionInWindow()
          }
          .graphicsLayer {
            alpha = buttonAlpha
            translationY = buttonTranslationY
            scaleX = fabScale.value
            scaleY = fabScale.value
          }
          .then(if (navVisible) Modifier else Modifier.pointerIgnore())
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
          .border(1.dp, AppTheme.colors.borderDefault, CircleShape)
          .then(
            if (hasMenu) {
              Modifier.pointerInput(navVisible, config) {
                awaitEachGesture {
                  val down = awaitFirstDown(requireUnconsumed = false)
                  if (!navVisible) {
                    return@awaitEachGesture
                  }

                  isMenuPressed = true
                  down.consume()
                  if (isMenuOpen) {
                    isMenuOpen = false
                    resetTrackedPointer()
                  } else {
                    trackedPointerOrigin = buttonWindowTopLeft + down.position
                    trackedPointerPosition = trackedPointerOrigin
                    isTrackedPointerArmed = false
                    isTrackedPointerHoldComplete = false
                    menuPointerState = AnchorPointerState(
                      position = trackedPointerOrigin!!,
                      isSelectionArmed = false,
                      isUp = false,
                    )
                    isMenuOpen = true

                    selectionArmJob?.cancel()
                    selectionArmJob = gestureScope.launch {
                      delay(FAB_SELECTION_ARM_DELAY_MS)
                      isTrackedPointerHoldComplete = true
                      trackedPointerPosition?.let { updateTrackedPointerArmState(it) }
                    }
                  }

                  while (true) {
                    val event = awaitPointerEvent()
                    val change = event.changes.find { it.id == down.id } ?: break
                    val windowPosition = buttonWindowTopLeft + change.position
                    trackedPointerPosition = windowPosition
                    updateTrackedPointerArmState(windowPosition)
                    menuPointerState = AnchorPointerState(
                      position = windowPosition,
                      isSelectionArmed = isTrackedPointerArmed,
                      isUp = !change.pressed,
                    )
                    if (isTrackedPointerArmed) change.consume()
                    if (!change.pressed) {
                      selectionArmJob?.cancel()
                      selectionArmJob = null
                      trackedPointerOrigin = null
                      trackedPointerPosition = null
                      isTrackedPointerArmed = false
                      isTrackedPointerHoldComplete = false
                      isMenuPressed = false
                      break
                    }
                  }

                  isMenuPressed = false
                }
              }
            } else {
              Modifier.clickable { config?.onClick?.invoke() }
            }
          ),
        contentAlignment = Alignment.Center,
      ) {
        Icon(
          icon = if (hasMenu && isMenuOpen) Lucide.X else (config?.icon ?: Lucide.SquarePen),
          tint = AppTheme.colors.textSecondary,
        )
      }
    }
  }
}

@Composable
private fun FabMenuItemRow(
  item: FabMenuItem,
  modifier: Modifier = Modifier,
) {
  Row(
    modifier = modifier,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(
      icon = item.icon,
      modifier = Modifier.size(18.dp),
      tint = item.tint ?: AppTheme.colors.textPrimary,
    )

    Spacer(Modifier.width(12.dp))

    Text(
      text = item.label,
      style = AppTheme.typography.action,
      color = item.tint ?: AppTheme.colors.textPrimary,
      maxLines = 1,
      overflow = TextOverflow.Ellipsis,
    )
  }
}
