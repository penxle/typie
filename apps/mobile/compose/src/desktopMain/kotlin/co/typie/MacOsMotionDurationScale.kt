package co.typie

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.MotionDurationScale
import androidx.compose.ui.window.ApplicationScope
import androidx.compose.ui.window.awaitApplication
import com.sun.jna.NativeLibrary
import com.sun.jna.Pointer
import java.awt.AWTEvent
import java.awt.Toolkit
import java.awt.event.AWTEventListener
import java.awt.event.WindowEvent
import kotlin.system.exitProcess
import kotlinx.coroutines.runBlocking

internal fun runDesktopApplication(content: @Composable ApplicationScope.() -> Unit) {
  val motionDurationScale = MacOsMotionDurationScale()
  try {
    runBlocking(motionDurationScale) { awaitApplication(content) }
  } finally {
    motionDurationScale.close()
  }
  exitProcess(0)
}

private class MacOsMotionDurationScale : MotionDurationScale, AutoCloseable {
  private var durationScale by mutableFloatStateOf(resolveDurationScale())
  private var listening = false
  private val activationListener = AWTEventListener { event ->
    if (event.id == WindowEvent.WINDOW_ACTIVATED) {
      durationScale = resolveDurationScale()
    }
  }

  override val scaleFactor: Float
    get() = durationScale

  init {
    if (MacOsReducedMotion.isSupported) {
      runCatching {
          Toolkit.getDefaultToolkit()
            .addAWTEventListener(activationListener, AWTEvent.WINDOW_EVENT_MASK)
        }
        .onSuccess { listening = true }
    }
  }

  override fun close() {
    if (!listening) return
    runCatching { Toolkit.getDefaultToolkit().removeAWTEventListener(activationListener) }
    listening = false
  }

  private fun resolveDurationScale(): Float =
    if (MacOsReducedMotion.shouldReduceMotion()) 0f else 1f
}

private object MacOsReducedMotion {
  val isSupported: Boolean = System.getProperty("os.name").startsWith("Mac", ignoreCase = true)

  fun shouldReduceMotion(): Boolean {
    if (!isSupported) return false

    return runCatching {
        NativeLibrary.getInstance("AppKit")
        val objectiveC = NativeLibrary.getInstance("objc")
        val getClass = objectiveC.getFunction("objc_getClass")
        val registerSelector = objectiveC.getFunction("sel_registerName")
        val sendMessage = objectiveC.getFunction("objc_msgSend")
        fun selector(name: String) =
          registerSelector.invoke(Pointer::class.java, arrayOf(name)) as Pointer

        val workspaceClass =
          getClass.invoke(Pointer::class.java, arrayOf("NSWorkspace")) as? Pointer
            ?: return@runCatching false
        val workspace =
          sendMessage.invoke(
            Pointer::class.java,
            arrayOf(workspaceClass, selector("sharedWorkspace")),
          ) as? Pointer ?: return@runCatching false

        sendMessage.invokeInt(
          arrayOf(workspace, selector("accessibilityDisplayShouldReduceMotion"))
        ) != 0
      }
      .getOrDefault(false)
  }
}
