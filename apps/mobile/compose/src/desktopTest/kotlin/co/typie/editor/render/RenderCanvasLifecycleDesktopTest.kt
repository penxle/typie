package co.typie.editor.render

import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.v2.runComposeUiTest
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class RenderCanvasLifecycleDesktopTest {
  @Test
  fun replacingOwnerDetachesOldCanvasBeforeAttachingNewCanvas() = runComposeUiTest {
    val first = Any()
    val second = Any()
    val events = mutableListOf<String>()
    var owner by mutableStateOf(first)

    setContent {
      val currentOwner = owner
      val ownerName = if (currentOwner === first) "first" else "second"
      RenderCanvasLifecycle(owner = currentOwner, page = 0) {
        DisposableEffect(Unit) {
          events += "attach:$ownerName"
          onDispose { events += "detach:$ownerName" }
        }
      }
    }
    waitForIdle()
    assertEquals(listOf("attach:first"), events)

    runOnIdle { owner = second }
    waitForIdle()

    assertEquals(listOf("attach:first", "detach:first", "attach:second"), events)
  }

  @Test
  fun replacingPageDetachesOldCanvasBeforeAttachingNewCanvas() = runComposeUiTest {
    val owner = Any()
    val events = mutableListOf<String>()
    var page by mutableStateOf(0)

    setContent {
      val currentPage = page
      RenderCanvasLifecycle(owner = owner, page = currentPage) {
        DisposableEffect(Unit) {
          events += "attach:$currentPage"
          onDispose { events += "detach:$currentPage" }
        }
      }
    }
    waitForIdle()
    assertEquals(listOf("attach:0"), events)

    runOnIdle { page = 1 }
    waitForIdle()

    assertEquals(listOf("attach:0", "detach:0", "attach:1"), events)
  }

  @Test
  fun replacingInstanceDetachesFailedCanvasBeforeAttachingReplacement() = runComposeUiTest {
    val owner = Any()
    val events = mutableListOf<String>()
    var instance by mutableStateOf(0)

    setContent {
      val currentInstance = instance
      RenderCanvasLifecycle(owner = owner, page = 0, instance = currentInstance) {
        DisposableEffect(Unit) {
          events += "attach:$currentInstance"
          onDispose { events += "detach:$currentInstance" }
        }
      }
    }
    waitForIdle()
    assertEquals(listOf("attach:0"), events)

    runOnIdle { instance = 1 }
    waitForIdle()

    assertEquals(listOf("attach:0", "detach:0", "attach:1"), events)
  }
}
