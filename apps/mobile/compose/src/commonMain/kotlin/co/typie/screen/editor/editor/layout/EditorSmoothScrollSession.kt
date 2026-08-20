package co.typie.screen.editor.editor.layout

import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.MotionDurationScale
import co.typie.editor.scroll.EditorBringIntoViewBehavior
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.viewport.EditorSmoothScrollMotion
import co.typie.editor.viewport.EditorViewportState
import kotlin.coroutines.coroutineContext
import kotlin.math.abs
import kotlin.math.sign
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

internal enum class EditorSmoothScrollUpdate {
  Unchanged,
  Changed,
  Finished,
}

internal class EditorSmoothScrollSession(
  private val coroutineScope: CoroutineScope,
  private val viewportState: EditorViewportState,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
) {
  private var job: Job? = null
  private var requestLifecycleJob: Job? = null
  private var request: EditorBringIntoViewRequests.Request? = null
  private var motion: EditorSmoothScrollMotion? = null
  private var acceptCompletion: ((EditorBringIntoViewRequests.Request) -> Boolean)? = null

  val active: Boolean
    get() = motion != null

  private fun reconcilePendingRequest(request: EditorBringIntoViewRequests.Request?) {
    if (motion == null || this.request === request) return
    val retainMotion = request?.behavior == EditorBringIntoViewBehavior.Smooth
    stop(clearMotion = !retainMotion)
    if (retainMotion) observeRequest(request)
  }

  fun retarget(
    request: EditorBringIntoViewRequests.Request,
    targetY: Float,
    viewportHeight: Float,
    maximumScrollY: Float,
    acceptCompletion: (EditorBringIntoViewRequests.Request) -> Boolean,
  ): EditorSmoothScrollUpdate {
    val position = viewportState.scrollOffset.y
    val target = targetY.coerceIn(0f, maximumScrollY)
    val previousMotion = motion
    val previous = previousMotion?.snapshot()
    if (
      this.request === request && job?.isActive == true && previous?.target == target.toDouble()
    ) {
      this.acceptCompletion = acceptCompletion
      return EditorSmoothScrollUpdate.Unchanged
    }

    val direction = (target - position).toDouble().sign
    val previousDirection = previous?.let { (it.target - it.position).sign } ?: 0.0
    motion =
      if (
        previousMotion == null ||
          (this.request !== request &&
            direction != 0.0 &&
            previousDirection != 0.0 &&
            direction != previousDirection)
      ) {
        EditorSmoothScrollMotion.start(
          position = position.toDouble(),
          target = target.toDouble(),
          viewportHeight = viewportHeight.toDouble(),
        )
      } else {
        previousMotion.apply {
          synchronizeBounds(
            actualPosition = position.toDouble(),
            maximumScroll = maximumScrollY.toDouble(),
            viewportHeight = viewportHeight.toDouble(),
          )
          retarget(target = target.toDouble(), viewportHeight = viewportHeight.toDouble())
        }
      }
    observeRequest(request)
    this.acceptCompletion = acceptCompletion

    if (motion?.finished == true && job?.isActive != true) {
      stop()
      return EditorSmoothScrollUpdate.Finished
    }
    if (job?.isActive != true) {
      job = coroutineScope.launch { run() }
    }
    return EditorSmoothScrollUpdate.Changed
  }

  fun translate(delta: Float) {
    motion?.translate(delta.toDouble())
  }

  fun finishIfNearTarget(request: EditorBringIntoViewRequests.Request): Boolean {
    val target = motion?.snapshot()?.target ?: return false
    val current = viewportState.scrollOffset.y
    val sameRequest = this.request === request
    val nearTarget = abs(target - current) <= 1.0
    if (!sameRequest || !nearTarget) {
      return false
    }
    stop()
    return true
  }

  fun stop(clearMotion: Boolean = true): EditorBringIntoViewRequests.Request? {
    val stoppedJob = job
    val stoppedRequestLifecycleJob = requestLifecycleJob
    val stoppedRequest = request
    job = null
    requestLifecycleJob = null
    request = null
    acceptCompletion = null
    if (clearMotion) {
      motion?.cancel()
      motion = null
    }
    stoppedJob?.cancel()
    stoppedRequestLifecycleJob?.cancel()
    return stoppedRequest
  }

  private fun observeRequest(request: EditorBringIntoViewRequests.Request) {
    if (this.request === request && requestLifecycleJob?.isActive == true) return
    requestLifecycleJob?.cancel()
    this.request = request
    requestLifecycleJob = coroutineScope.launch {
      bringIntoViewRequests.awaitPresentation(request)
      if (requestLifecycleJob == coroutineContext[Job]) {
        requestLifecycleJob = null
      }
      if (this@EditorSmoothScrollSession.request === request) {
        reconcilePendingRequest(bringIntoViewRequests.pendingRequest)
      }
    }
  }

  private suspend fun run() {
    var completed = false
    try {
      var previousFrameTime: Long? = null
      while (true) {
        val currentMotion = motion ?: return
        if (currentMotion.finished) break
        val frameTime = withFrameNanos { it }
        val durationScale = motionDurationScale()
        if (durationScale == 0.0) break
        val previousTime = previousFrameTime
        previousFrameTime = frameTime
        if (previousTime == null) continue
        val activeMotion = motion ?: return
        if (activeMotion.finished) continue
        val viewportHeight = viewportState.viewportSize.height
        val maximumScrollY = viewportState.maxScrollY
        val position = viewportState.scrollOffset.y
        val snapshot = activeMotion.snapshot()
        if (
          abs(snapshot.position - position) > 1.0 ||
            snapshot.target < 0.0 ||
            snapshot.target > maximumScrollY.toDouble()
        ) {
          activeMotion.synchronizeBounds(
            actualPosition = position.toDouble(),
            maximumScroll = maximumScrollY.toDouble(),
            viewportHeight = viewportHeight.toDouble(),
          )
        }
        val next =
          activeMotion.advance((frameTime - previousTime) / 1_000_000_000.0 / durationScale)
        viewportState.scrollToY(targetY = next.position.toFloat(), isAutoScroll = true)
      }
      motion?.snapshot()?.target?.let { target ->
        viewportState.scrollToY(targetY = target.toFloat(), isAutoScroll = true)
      }
      completed = true
    } finally {
      if (job == coroutineContext[Job]) {
        val completedRequest = request
        val completion = acceptCompletion
        job = null
        val retainMotion =
          completed && completedRequest != null && completion?.invoke(completedRequest) == false
        if (!retainMotion) {
          requestLifecycleJob?.cancel()
          requestLifecycleJob = null
          request = null
          motion = null
          acceptCompletion = null
        }
      }
    }
  }

  private suspend fun motionDurationScale(): Double {
    val scale = coroutineContext[MotionDurationScale]?.scaleFactor ?: 1f
    return scale.takeIf { it.isFinite() && it >= 0f }?.toDouble() ?: 1.0
  }
}
