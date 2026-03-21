@file:OptIn(ExperimentalTime::class)

package co.typie.dev

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.datetime.format
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.delay
import java.awt.MouseInfo
import java.awt.Point
import java.awt.Window
import java.util.prefs.Preferences
import kotlin.time.Clock
import kotlin.time.ExperimentalTime

// iPhone 16 Pro Max
private val StatusBarHeight = 59.dp
private val DynamicIslandWidth = 126.dp
private val DynamicIslandHeight = 37.dp
private val DynamicIslandTop = 11.dp
private val HomeIndicatorWidth = 134.dp
private val HomeIndicatorHeight = 5.dp
private val HomeIndicatorBottom = 8.dp
private val ScreenCornerRadius = 55.dp

// Bezel layers (outer → inner)
private val BezelOuterHighlight = Color(0xFF6E6E73) // edge reflection
private val BezelBody = Color(0xFF1C1C1E)           // main frame
private val BezelInnerEdge = Color(0xFF3A3A3C)      // inner chamfer
private val BezelScreenEdge = Color(0xFF000000)      // screen-to-frame seam

@Composable
actual fun SystemChrome(content: @Composable () -> Unit) {
  val r = ScreenCornerRadius

  Box(
    Modifier
      .fillMaxSize()
      // outer highlight (1.5dp)
      .background(BezelOuterHighlight, RoundedCornerShape(r + 12.dp))
      .padding(1.5.dp)
      // main bezel body (9dp)
      .background(BezelBody, RoundedCornerShape(r + 10.5.dp))
      .padding(9.dp)
      // inner chamfer (1dp)
      .background(BezelInnerEdge, RoundedCornerShape(r + 1.5.dp))
      .padding(1.dp)
      // screen-to-frame seam (0.5dp)
      .background(BezelScreenEdge, RoundedCornerShape(r + 0.5.dp))
      .padding(0.5.dp),
  ) {
    Box(Modifier.fillMaxSize().clip(RoundedCornerShape(r))) {
      content()
      StatusBar(Modifier.fillMaxWidth().align(Alignment.TopStart))
      HomeIndicator(Modifier.align(Alignment.BottomCenter))
    }
  }
}

@Composable
private fun StatusBar(modifier: Modifier = Modifier) {
  val contentColor = AppTheme.colors.textDefault
  var time by remember { mutableStateOf(Clock.System.now().format("HH:mm")) }

  LaunchedEffect(Unit) {
    while (true) {
      time = Clock.System.now().format("HH:mm")
      delay(60_000 - (System.currentTimeMillis() % 60_000))
    }
  }

  Box(
    modifier
      .height(StatusBarHeight)
      .pointerInput(Unit) {
        var dragStart = Point()
        var windowStart = Point()
        awaitPointerEventScope {
          while (true) {
            val event = awaitPointerEvent()
            val awtWindow = Window.getWindows().firstOrNull() ?: continue
            when {
              event.changes.any { it.pressed && !it.previousPressed } -> {
                dragStart = MouseInfo.getPointerInfo().location
                windowStart = awtWindow.location
              }

              event.changes.any { it.pressed } -> {
                val mouse = MouseInfo.getPointerInfo().location
                awtWindow.location = Point(
                  windowStart.x + mouse.x - dragStart.x,
                  windowStart.y + mouse.y - dragStart.y,
                )
              }
              // drag ended — save position
              event.changes.any { !it.pressed && it.previousPressed } -> {
                val prefs = Preferences.userRoot().node("co/typie")
                prefs.putInt("windowX", awtWindow.x)
                prefs.putInt("windowY", awtWindow.y)
                prefs.flush()
              }
            }
            event.changes.forEach { it.consume() }
          }
        }
      },
  ) {
    // Dynamic Island
    Box(
      Modifier
        .align(Alignment.TopCenter)
        .padding(top = DynamicIslandTop)
        .size(DynamicIslandWidth, DynamicIslandHeight)
        .background(Color.Black, CircleShape),
    )

    // Status bar content (vertically centered with the dynamic island)
    Row(
      Modifier
        .fillMaxWidth()
        .padding(top = DynamicIslandTop)
        .height(DynamicIslandHeight)
        .padding(horizontal = 30.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      // Time
      BasicText(
        text = time,
        style = TextStyle(
          fontSize = 16.sp,
          fontWeight = FontWeight.SemiBold,
          color = contentColor,
        ),
      )

      Spacer(Modifier.weight(1f))

      // Signal + Battery
      Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
      ) {
        SignalBars(contentColor)
        BatteryIcon(contentColor)
      }
    }
  }
}

@Composable
private fun SignalBars(color: Color) {
  Row(
    horizontalArrangement = Arrangement.spacedBy(1.5.dp),
    verticalAlignment = Alignment.Bottom,
  ) {
    listOf(4.dp, 6.5.dp, 9.dp, 12.dp).forEach { h ->
      Box(
        Modifier
          .width(3.dp)
          .height(h)
          .background(color, RoundedCornerShape(0.5.dp)),
      )
    }
  }
}

@Composable
private fun BatteryIcon(color: Color) {
  Row(verticalAlignment = Alignment.CenterVertically) {
    Box(
      Modifier
        .size(25.dp, 12.dp)
        .border(1.5.dp, color, RoundedCornerShape(3.dp))
        .padding(2.5.dp),
    ) {
      Box(
        Modifier
          .fillMaxHeight()
          .fillMaxWidth(0.8f)
          .background(color, RoundedCornerShape(1.dp)),
      )
    }
    Box(
      Modifier
        .padding(start = 1.dp)
        .size(2.dp, 5.dp)
        .background(color, RoundedCornerShape(topEnd = 1.dp, bottomEnd = 1.dp)),
    )
  }
}

@Composable
private fun HomeIndicator(modifier: Modifier = Modifier) {
  val color = AppTheme.colors.textDefault
  Box(
    modifier
      .padding(bottom = HomeIndicatorBottom)
      .size(HomeIndicatorWidth, HomeIndicatorHeight)
      .background(color.copy(alpha = 0.6f), CircleShape),
  )
}
