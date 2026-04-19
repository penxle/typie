package co.typie.shell

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.DraggableAnchors
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.anchoredDraggable
import androidx.compose.foundation.gestures.animateTo
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.horizontalDrag
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.ext.statusBarsPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.navigation.PlatformBackHandler
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Divider
import co.typie.ui.component.Img
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.drawer.Drawer
import co.typie.ui.component.drawer.DrawerAnchor
import co.typie.ui.component.drawer.DrawerDefaults
import co.typie.ui.component.drawer.LocalDrawer
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

@Composable
internal fun MainDrawerContent() {
  val model = viewModel { MainDrawerViewModel() }
  val selectedSiteId = Preference.siteId

  LaunchedEffect(selectedSiteId) {
    if (!selectedSiteId.isNullOrBlank()) {
      model.query.refetch()
    }
  }

  val nav = Nav.current
  val uriHandler = LocalUriHandler.current
  val scope = rememberCoroutineScope()
  val sheet = LocalSheet.current
  val drawer = LocalDrawer.current

  // Fire close animation and the follow-up action in two independent coroutines.
  // AnchoredDraggableState.animateTo uses a MutatorMutex that cancels the in-flight
  // animate-coroutine when another caller (e.g. scrim tap) re-invokes animateTo —
  // if we chained close→action in one coroutine the CancellationException from the
  // preempted animate would also abort the action.
  val dismissAndRun: (suspend () -> Unit) -> Unit = { action ->
    scope.launch { drawer.close() }
    scope.launch { action() }
  }

  Skeleton(enabled = model.query.state !is QueryState.Success) {
    val data = model.query.data
    val availableSiteIds = data.me.sites.map { it.id }
    val selection =
      resolveMainDrawerSelection(
        selectedSiteId = Preference.siteId.orEmpty(),
        availableSiteIds = availableSiteIds,
      )
    val currentSite = data.me.sites.first { it.id == selection.currentSiteId }

    if (model.query.state is QueryState.Success) {
      val pendingCreatedSiteId =
        resolvePendingCreatedSiteSelection(
          pendingCreatedSiteId = model.pendingCreatedSiteId,
          availableSiteIds = availableSiteIds,
        )

      if (pendingCreatedSiteId != null) {
        LaunchedEffect(pendingCreatedSiteId) {
          Preference.siteId = pendingCreatedSiteId
          model.consumePendingCreatedSiteSelection(pendingCreatedSiteId)
        }
      } else if (selection.currentSiteId != Preference.siteId) {
        LaunchedEffect(selection.currentSiteId) { Preference.siteId = selection.currentSiteId }
      }
    }

    Column(modifier = Modifier.fillMaxHeight()) {
      val scrollState = rememberScrollState()

      Column(
        modifier =
          Modifier.weight(1f)
            .verticalScroll(scrollState)
            .padding(horizontal = 4.dp, vertical = 8.dp)
      ) {
        Text(
          text = "스페이스",
          style = AppTheme.typography.title,
          color = AppTheme.colors.textDefault,
          modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
        )

        Spacer(Modifier.height(4.dp))

        for (site in data.me.sites) {
          val isCurrent = site.id == selection.currentSiteId

          InteractionScope {
            Row(
              verticalAlignment = Alignment.CenterVertically,
              modifier =
                Modifier.fillMaxWidth()
                  .background(
                    if (isCurrent) AppTheme.colors.surfaceInset else AppTheme.colors.surfaceDefault,
                    AppShapes.rounded(AppShapes.md),
                  )
                  .clickable {
                    if (site.id != selection.currentSiteId) {
                      Preference.siteId = site.id
                    }

                    scope.launch { drawer.close() }
                  }
                  .pressScale()
                  .padding(horizontal = 12.dp, vertical = 12.dp),
            ) {
              Box(
                modifier =
                  Modifier.size(44.dp)
                    .border(
                      2.dp,
                      if (isCurrent) AppTheme.colors.textDefault
                      else AppTheme.colors.textDefault.copy(alpha = 0f),
                      AppShapes.rounded(AppShapes.md),
                    )
                    .padding(4.dp)
              ) {
                Img(
                  image = site.logo.img_image,
                  modifier = Modifier.fillMaxSize().clip(AppShapes.rounded(AppShapes.md - 4.dp)),
                )
              }

              Spacer(Modifier.width(8.dp))

              Column(modifier = Modifier.weight(1f)) {
                Text(
                  text = site.name,
                  style = AppTheme.typography.label,
                  color = AppTheme.colors.textDefault,
                  maxLines = 1,
                  overflow = TextOverflow.Ellipsis,
                )

                Text(
                  text = site.url,
                  style = AppTheme.typography.micro,
                  color = AppTheme.colors.textHint,
                  maxLines = 1,
                  overflow = TextOverflow.Ellipsis,
                )
              }
            }
          }
        }

        Row(
          verticalAlignment = Alignment.CenterVertically,
          modifier =
            Modifier.fillMaxWidth()
              .clickable { dismissAndRun { sheet.present { CreateSpaceSheet(model) } } }
              .padding(horizontal = 12.dp, vertical = 12.dp),
        ) {
          Box(
            modifier =
              Modifier.padding(start = 4.dp)
                .size(36.dp)
                .border(1.dp, AppTheme.colors.borderEmphasis, AppShapes.rounded(AppShapes.md)),
            contentAlignment = Alignment.Center,
          ) {
            Icon(
              icon = Lucide.Plus,
              tint = AppTheme.colors.textMuted,
              modifier = Modifier.size(20.dp),
            )
          }

          Spacer(Modifier.width(12.dp))

          Text(
            text = "새 스페이스 생성",
            style = AppTheme.typography.action,
            color = AppTheme.colors.textMuted,
          )
        }
      }

      Divider(inset = 16.dp)

      Spacer(Modifier.height(4.dp))

      Column(modifier = Modifier.padding(horizontal = 8.dp)) {
        DrawerActionRow(icon = Lucide.Settings, label = "스페이스 설정") {
          dismissAndRun { nav.navigate(Route.SpaceSettings) }
        }

        DrawerActionRow(icon = Lucide.ExternalLink, label = "스페이스 열기") {
          dismissAndRun { uriHandler.openUri(currentSite.url) }
        }

        DrawerActionRow(icon = Lucide.Trash2, label = "휴지통") {
          dismissAndRun { nav.navigate(Route.Trash()) }
        }
      }

      Spacer(Modifier.height(4.dp))

      Divider(inset = 16.dp)

      Spacer(Modifier.height(4.dp))

      Column(modifier = Modifier.padding(horizontal = 8.dp)) {
        DrawerActionRow(icon = Lucide.StickyNote, label = "노트") {
          dismissAndRun { nav.navigate(Route.Notes) }
        }

        DrawerActionRow(icon = Lucide.Settings, label = "설정") {
          dismissAndRun { nav.navigate(Route.Settings) }
        }

        DrawerActionRow(icon = Lucide.Ellipsis, label = "더 보기") {
          dismissAndRun { nav.navigate(Route.More) }
        }
      }
    }
  }
}

@Composable
private fun DrawerActionRow(icon: IconData, label: String, onClick: () -> Unit) {
  InteractionScope {
    Row(
      verticalAlignment = Alignment.CenterVertically,
      modifier =
        Modifier.fillMaxWidth()
          .clickable { onClick() }
          .pressScale()
          .padding(horizontal = 12.dp, vertical = 12.dp),
    ) {
      Icon(icon, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textMuted)

      Spacer(Modifier.width(12.dp))

      Text(text = label, style = AppTheme.typography.action, color = AppTheme.colors.textMuted)
    }
  }
}

@Composable
context(_: SheetScope<Unit>)
internal fun CreateSpaceSheet(model: MainDrawerViewModel) {
  var name by remember { mutableStateOf("") }
  val toast = LocalToast.current

  SheetLayout(
    bodyScroll = false,
    header = {
      SheetBar(
        center = {
          Text(
            text = "새 스페이스 생성",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    },
  ) {
    Text(
      text = "스페이스는 독립된 글쓰기 공간이에요.\n주제나 목적에 따라 글을 나누어 관리해보세요.",
      style = AppTheme.typography.body,
      color = AppTheme.colors.textMuted,
    )

    TextField(
      value = name,
      onValueChange = { name = it },
      label = "스페이스 이름",
      labelPosition = LabelPosition.External,
      placeholder = "새 스페이스",
      autoFocus = true,
    )

    Row(horizontalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
      Button(
        text = "취소",
        variant = ButtonVariant.Secondary,
        enabled = !model.isCreatingSite,
        onClick = { dismiss() },
        modifier = Modifier.weight(1f),
      )

      Button(
        text = "생성",
        loading = model.isCreatingSite,
        enabled = !model.isCreatingSite,
        onClick = {
          model.createSite(name).withDefaultExceptionHandler(toast).onOk {
            toast.show(ToastType.Success, "새 스페이스가 생성되었어요.")
            dismiss()
          }
        },
        modifier = Modifier.weight(1f),
      )
    }
  }
}

@Composable
fun MainDrawerOverlay(drawer: Drawer) {
  val density = LocalDensity.current
  val scope = rememberCoroutineScope()
  val shadowSpot = AppTheme.colors.shadowSpot

  BoxWithConstraints(modifier = Modifier.fillMaxSize()) {
    val panelWidthDp =
      minOf(maxWidth.value * DrawerDefaults.WidthFraction, DrawerDefaults.MaxWidth.value).dp
    val panelWidthPx = with(density) { panelWidthDp.toPx() }

    remember(panelWidthPx) {
      // Pass currentValue (not targetValue) as the snap target. Initial placeholder anchors
      // in Drawer() are { Closed at 0f, Open at 0f }; with both at 0f, targetValue —
      // derived via anchors.closestAnchor(offset) — tie-breaks to Open (last-declared wins
      // in DraggableAnchors iteration). Feeding that Open into updateAnchors would snap
      // offset to the new Open anchor (0f) and flip currentValue to Open on first frame,
      // which then trips the close animation on the next tick.
      drawer.state.updateAnchors(
        DraggableAnchors {
          DrawerAnchor.Closed at -panelWidthPx
          DrawerAnchor.Open at 0f
        },
        drawer.state.currentValue,
      )
    }

    val rawOffset = if (drawer.state.offset.isNaN()) -panelWidthPx else drawer.state.offset
    val progress =
      if (panelWidthPx == 0f) 0f else ((rawOffset + panelWidthPx) / panelWidthPx).coerceIn(0f, 1f)

    if (progress > 0f) {
      Box(
        modifier =
          Modifier.fillMaxSize()
            .background(AppTheme.colors.scrim.copy(alpha = progress * DrawerDefaults.ScrimAlpha))
            .clickable { drawer.close() }
      )
    }

    PlatformBackHandler(enabled = drawer.isOpen) { scope.launch { drawer.close() } }

    val panelShape = RoundedCornerShape(topEnd = AppShapes.xl, bottomEnd = AppShapes.xl)

    Column(
      modifier =
        Modifier.align(Alignment.CenterStart)
          .zIndex(1f)
          .fillMaxHeight()
          .width(panelWidthDp)
          .offset { IntOffset(rawOffset.roundToInt(), 0) }
          .anchoredDraggable(
            state = drawer.state,
            orientation = Orientation.Horizontal,
            enabled = !drawer.isProgrammaticAnimating,
          )
          .dropShadow(panelShape) {
            color = shadowSpot
            offset = Offset(4f, 0f)
            radius = 24f
          }
          .background(AppTheme.colors.surfaceDefault, panelShape)
          .statusBarsPadding()
          .navigationBarsPadding()
    ) {
      MainDrawerContent()
    }
  }
}

@Composable
fun MainDrawerEdgeGestureHost(drawer: Drawer) {
  val scope = rememberCoroutineScope()

  Box(
    modifier =
      Modifier.fillMaxHeight().width(DrawerDefaults.EdgeHitSlop).pointerInput(drawer) {
        awaitEachGesture {
          val down = awaitFirstDown(requireUnconsumed = false)
          if (drawer.isOpen) return@awaitEachGesture

          horizontalDrag(down.id) { change ->
            val dx = change.positionChange().x
            if (dx != 0f) {
              drawer.state.dispatchRawDelta(dx)
              change.consume()
            }
          }
          scope.launch {
            drawer.state.animateTo(drawer.state.targetValue, DrawerDefaults.AnimationSpec)
          }
        }
      }
  )
}
