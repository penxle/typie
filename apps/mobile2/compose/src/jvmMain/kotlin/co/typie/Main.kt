package co.typie

import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.type
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Window
import androidx.compose.ui.window.WindowState
import androidx.compose.ui.window.application
import co.typie.di.initKoin
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.NativeLibrary
import com.sun.jna.Pointer
import com.sun.jna.Structure
import java.awt.Taskbar
import java.awt.Toolkit
import javax.imageio.ImageIO
import javax.swing.SwingUtilities

// iPhone 16 Pro Max: 440×956 pt, @3x, 460 PPI
private const val DEVICE_POINT_WIDTH = 440
private const val DEVICE_POINT_HEIGHT = 956
private const val DEVICE_PPI = 460
private const val DEVICE_PIXEL_SCALE = 3

@Structure.FieldOrder("width", "height")
open class CGSize : Structure(), Structure.ByValue {
  @JvmField
  var width: Double = 0.0

  @JvmField
  var height: Double = 0.0
}

private interface CoreGraphics : Library {
  fun CGMainDisplayID(): Int
  fun CGDisplayScreenSize(display: Int): CGSize
}

private val coreGraphics: CoreGraphics? = runCatching {
  Native.load("CoreGraphics", CoreGraphics::class.java)
}.getOrNull()

private fun physicalSizeScale(): Double {
  val deviceWidthInch = DEVICE_POINT_WIDTH.toDouble() * DEVICE_PIXEL_SCALE / DEVICE_PPI
  val monitorWidthMm = coreGraphics?.let {
    it.CGDisplayScreenSize(it.CGMainDisplayID()).width
  } ?: return 0.80
  val monitorLogicalPpi = Toolkit.getDefaultToolkit().screenSize.width / (monitorWidthMm / 25.4)
  return deviceWidthInch * monitorLogicalPpi / DEVICE_POINT_WIDTH
}

private fun disableWindowFullScreen() {
  try {
    val objcLib = NativeLibrary.getInstance("objc")
    val objcGetClass = objcLib.getFunction("objc_getClass")
    val selRegisterName = objcLib.getFunction("sel_registerName")
    val msgSend = objcLib.getFunction("objc_msgSend")

    fun sel(name: String) = selRegisterName.invoke(Pointer::class.java, arrayOf(name)) as Pointer
    fun msg(receiver: Pointer, selName: String, vararg args: Any) =
      msgSend.invoke(Pointer::class.java, arrayOf(receiver, sel(selName), *args)) as? Pointer

    val nsApp = msg(
      objcGetClass.invoke(Pointer::class.java, arrayOf("NSApplication")) as? Pointer ?: return,
      "sharedApplication",
    ) ?: return

    val windows = msg(nsApp, "windows") ?: return
    val nsWindow = msg(windows, "firstObject") ?: return

    // Disable zoom button (NSWindowZoomButton = 2)
    val zoomButton = msg(nsWindow, "standardWindowButton:", 2L) ?: return
    msg(zoomButton, "setEnabled:", 0L)

    // NSWindowCollectionBehaviorFullScreenNone (1 << 9) |
    // NSWindowCollectionBehaviorFullScreenDisallowsTiling (1 << 12)
    msg(nsWindow, "setCollectionBehavior:", (1L shl 9) or (1L shl 12))
  } catch (_: Exception) {
    // ignore on non-macOS
  }
}

fun main() {
  initKoin {
    printLogger()
  }

  if (Taskbar.isTaskbarSupported()) {
    Taskbar.getTaskbar().iconImage =
      ImageIO.read(Thread.currentThread().contextClassLoader.getResourceAsStream("icon.png"))
  }

  val physicalScale = physicalSizeScale()
  val pointAccurateSize = DpSize(DEVICE_POINT_WIDTH.dp, DEVICE_POINT_HEIGHT.dp)
  val physicalSize = DpSize(
    (DEVICE_POINT_WIDTH * physicalScale).dp,
    (DEVICE_POINT_HEIGHT * physicalScale).dp,
  )

  application {
    var usePhysicalScale by remember { mutableStateOf(true) }
    val windowState = remember { WindowState(size = physicalSize) }

    Window(
      onCloseRequest = ::exitApplication,
      alwaysOnTop = true,
      title = "Typie",
      state = windowState,
      onPreviewKeyEvent = { event ->
        if (event.isMetaPressed && event.type == KeyEventType.KeyDown) {
          when (event.key) {
            Key.One -> {
              windowState.size = physicalSize; usePhysicalScale = true; true
            }

            Key.Two -> {
              windowState.size = pointAccurateSize; usePhysicalScale = false; true
            }

            else -> false
          }
        } else {
          false
        }
      },
    ) {
      LaunchedEffect(Unit) {
        SwingUtilities.invokeLater { disableWindowFullScreen() }
      }
      val currentDensity = LocalDensity.current
      val scale = if (usePhysicalScale) physicalScale.toFloat() else 1f
      val adjustedDensity = remember(currentDensity, scale) {
        Density(
          density = currentDensity.density * scale,
          fontScale = currentDensity.fontScale,
        )
      }
      CompositionLocalProvider(LocalDensity provides adjustedDensity) {
        App()
      }
    }
  }
}
