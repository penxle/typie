package co.typie.platform

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.os.Build
import android.os.CancellationSignal
import android.view.View
import android.view.animation.PathInterpolator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalView
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsAnimationControlListenerCompat
import androidx.core.view.WindowInsetsAnimationControllerCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import kotlin.math.roundToInt

internal fun interface AndroidImeEndpointAnimation {
  fun cancel()
}

internal fun interface AndroidImeEndpointAnimator {
  fun animate(
    fromHiddenProgress: Float,
    toHiddenProgress: Float,
    onProgress: (Float) -> Unit,
    onFinished: () -> Unit,
  ): AndroidImeEndpointAnimation
}

private object ValueAnimatorAndroidImeEndpointAnimator : AndroidImeEndpointAnimator {
  override fun animate(
    fromHiddenProgress: Float,
    toHiddenProgress: Float,
    onProgress: (Float) -> Unit,
    onFinished: () -> Unit,
  ): AndroidImeEndpointAnimation {
    if (fromHiddenProgress == toHiddenProgress) {
      onProgress(toHiddenProgress)
      onFinished()
      return AndroidImeEndpointAnimation {}
    }

    var cancelled = false
    val animator =
      ValueAnimator.ofFloat(fromHiddenProgress, toHiddenProgress).apply {
        duration = AndroidImeEndpointAnimationDurationMillis
        interpolator = PathInterpolator(0.4f, 0f, 0.2f, 1f)
        addUpdateListener { onProgress(it.animatedValue as Float) }
        addListener(
          object : AnimatorListenerAdapter() {
            override fun onAnimationCancel(animation: Animator) {
              cancelled = true
            }

            override fun onAnimationEnd(animation: Animator) {
              if (!cancelled) onFinished()
            }
          }
        )
        start()
      }
    return AndroidImeEndpointAnimation(animator::cancel)
  }
}

internal interface AndroidImeAnimationControl {
  val hiddenStateInsets: Insets
  val shownStateInsets: Insets
  val currentAlpha: Float
  val isReady: Boolean

  fun setInsetsAndAlpha(insets: Insets, alpha: Float, fraction: Float)

  fun finish(shown: Boolean)
}

internal class AndroidSoftwareKeyboardPresentationDriver(
  private val onInvalidated: () -> Unit,
  private val cancelControl: () -> Unit,
  private val hideIme: () -> Unit,
  private val isImeVisible: () -> Boolean,
  private val endpointAnimator: AndroidImeEndpointAnimator = ValueAnimatorAndroidImeEndpointAnimator,
) : SoftwareKeyboardPresentationDriver {
  private sealed interface Lifecycle {
    class Pending(var hiddenProgress: Float) : Lifecycle

    class Ready(val control: AndroidImeAnimationControl, var hiddenProgress: Float) : Lifecycle

    class Terminal(
      val endpoint: SoftwareKeyboardPresentationEndpoint,
      val onAccepted: () -> Unit,
      var hiddenProgress: Float,
      var animation: AndroidImeEndpointAnimation? = null,
    ) : Lifecycle

    data object Closed : Lifecycle
  }

  private var lifecycle: Lifecycle = Lifecycle.Pending(hiddenProgress = 0f)

  override fun updateHiddenProgress(progress: Float) {
    val hiddenProgress = progress.coerceIn(0f, 1f)
    when (val current = lifecycle) {
      is Lifecycle.Pending -> current.hiddenProgress = hiddenProgress
      is Lifecycle.Ready -> {
        current.hiddenProgress = hiddenProgress
        applyHiddenProgress(current)
      }
      is Lifecycle.Terminal,
      Lifecycle.Closed -> Unit
    }
  }

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) {
    when (val current = lifecycle) {
      is Lifecycle.Pending ->
        lifecycle = Lifecycle.Terminal(endpoint, onAccepted, current.hiddenProgress)
      is Lifecycle.Ready -> {
        val terminal = Lifecycle.Terminal(endpoint, onAccepted, current.hiddenProgress)
        lifecycle = terminal
        animateControlToEndpoint(control = current.control, terminal = terminal)
      }
      is Lifecycle.Terminal,
      Lifecycle.Closed -> Unit
    }
  }

  override fun dispose() {
    val current = lifecycle
    if (current == Lifecycle.Closed) return
    lifecycle = Lifecycle.Closed
    if (current is Lifecycle.Terminal) current.animation?.cancel()
    cancelControl()
  }

  fun onCancelled() {
    when (val current = lifecycle) {
      is Lifecycle.Pending,
      is Lifecycle.Ready -> {
        lifecycle = Lifecycle.Closed
        onInvalidated()
      }
      is Lifecycle.Terminal -> {
        lifecycle = Lifecycle.Closed
        current.animation?.cancel()
        val accepted =
          runCatching {
              when (current.endpoint) {
                SoftwareKeyboardPresentationEndpoint.Shown -> isImeVisible()
                SoftwareKeyboardPresentationEndpoint.Hidden -> {
                  hideIme()
                  true
                }
              }
            }
            .getOrDefault(false)
        if (accepted) current.onAccepted() else onInvalidated()
      }
      Lifecycle.Closed -> Unit
    }
  }

  fun onReady(control: AndroidImeAnimationControl) {
    when (val current = lifecycle) {
      is Lifecycle.Pending -> {
        if (!control.isReady) {
          onCancelled()
          return
        }
        val ready = Lifecycle.Ready(control = control, hiddenProgress = current.hiddenProgress)
        lifecycle = ready
        if (runCatching { applyHiddenProgress(ready) }.isFailure && lifecycle === ready) {
          onCancelled()
        }
      }
      is Lifecycle.Terminal -> {
        if (!control.isReady) {
          onCancelled()
          return
        }
        if (
          runCatching { animateControlToEndpoint(control = control, terminal = current) }
            .isFailure && lifecycle === current
        ) {
          onCancelled()
        }
      }
      is Lifecycle.Ready,
      Lifecycle.Closed -> Unit
    }
  }

  fun onFinished() {
    when (val current = lifecycle) {
      is Lifecycle.Pending,
      is Lifecycle.Ready -> {
        lifecycle = Lifecycle.Closed
        onInvalidated()
      }
      is Lifecycle.Terminal -> {
        current.animation?.cancel()
        acceptTerminal(current)
      }
      Lifecycle.Closed -> Unit
    }
  }

  private fun animateControlToEndpoint(
    control: AndroidImeAnimationControl,
    terminal: Lifecycle.Terminal,
  ) {
    val animation =
      endpointAnimator.animate(
        fromHiddenProgress = terminal.hiddenProgress,
        toHiddenProgress = terminal.endpoint.hiddenProgress,
        onProgress = { hiddenProgress ->
          if (lifecycle !== terminal) return@animate
          terminal.hiddenProgress = hiddenProgress
          if (runCatching { applyHiddenProgress(control, hiddenProgress) }.isFailure) {
            onCancelled()
          }
        },
        onFinished = {
          if (lifecycle !== terminal) return@animate
          terminal.animation = null
          if (
            runCatching {
                control.finish(
                  shown = terminal.endpoint == SoftwareKeyboardPresentationEndpoint.Shown
                )
              }
              .isFailure
          ) {
            onCancelled()
          } else {
            acceptTerminal(terminal)
          }
        },
      )
    if (lifecycle === terminal) {
      terminal.animation = animation
    } else {
      animation.cancel()
    }
  }

  private fun acceptTerminal(terminal: Lifecycle.Terminal) {
    if (lifecycle !== terminal) return
    lifecycle = Lifecycle.Closed
    terminal.onAccepted()
  }

  private fun applyHiddenProgress(ready: Lifecycle.Ready) {
    applyHiddenProgress(ready.control, ready.hiddenProgress)
  }

  private fun applyHiddenProgress(control: AndroidImeAnimationControl, hiddenProgress: Float) {
    val shownFraction = 1f - hiddenProgress
    val shown = control.shownStateInsets
    val hidden = control.hiddenStateInsets
    control.setInsetsAndAlpha(
      insets =
        Insets.of(
          shown.left.interpolateTo(hidden.left, hiddenProgress),
          shown.top.interpolateTo(hidden.top, hiddenProgress),
          shown.right.interpolateTo(hidden.right, hiddenProgress),
          shown.bottom.interpolateTo(hidden.bottom, hiddenProgress),
        ),
      alpha = control.currentAlpha,
      fraction = shownFraction,
    )
  }

  private fun Int.interpolateTo(target: Int, fraction: Float): Int =
    (this + (target - this) * fraction).roundToInt()
}

private val SoftwareKeyboardPresentationEndpoint.hiddenProgress: Float
  get() = if (this == SoftwareKeyboardPresentationEndpoint.Hidden) 1f else 0f

private const val AndroidImeEndpointAnimationDurationMillis = 220L

private val UnavailableSoftwareKeyboardPresentationDriverFactory =
  SoftwareKeyboardPresentationDriverFactory {
    null
  }

@Composable
internal actual fun rememberSoftwareKeyboardPresentationDriverFactory():
  SoftwareKeyboardPresentationDriverFactory {
  if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
    return UnavailableSoftwareKeyboardPresentationDriverFactory
  }

  val activity = activityContext()
  val view = LocalView.current
  return remember(activity, view) {
    val insetsController = WindowCompat.getInsetsController(activity.window, view)
    SoftwareKeyboardPresentationDriverFactory { onInvalidated ->
      acquireAndroidSoftwareKeyboardPresentationDriver(
        view = view,
        insetsController = insetsController,
        onInvalidated = onInvalidated,
      )
    }
  }
}

private fun acquireAndroidSoftwareKeyboardPresentationDriver(
  view: View,
  insetsController: WindowInsetsControllerCompat,
  onInvalidated: () -> Unit,
): SoftwareKeyboardPresentationDriver? {
  val imeType = WindowInsetsCompat.Type.ime()
  if (
    !view.isAttachedToWindow ||
      !view.hasWindowFocus() ||
      ViewCompat.getRootWindowInsets(view)?.isVisible(imeType) != true
  ) {
    return null
  }

  val cancellationSignal = CancellationSignal()
  val driver =
    AndroidSoftwareKeyboardPresentationDriver(
      onInvalidated = onInvalidated,
      cancelControl = cancellationSignal::cancel,
      hideIme = { insetsController.hide(imeType) },
      isImeVisible = { ViewCompat.getRootWindowInsets(view)?.isVisible(imeType) == true },
    )
  val listener =
    object : WindowInsetsAnimationControlListenerCompat {
      override fun onReady(controller: WindowInsetsAnimationControllerCompat, types: Int) {
        if (types and imeType == 0) {
          cancellationSignal.cancel()
          driver.onCancelled()
        } else {
          driver.onReady(AndroidImeAnimationControlCompat(controller))
        }
      }

      override fun onFinished(controller: WindowInsetsAnimationControllerCompat) {
        driver.onFinished()
      }

      override fun onCancelled(controller: WindowInsetsAnimationControllerCompat?) {
        driver.onCancelled()
      }
    }

  try {
    insetsController.controlWindowInsetsAnimation(imeType, -1L, null, cancellationSignal, listener)
  } catch (throwable: Throwable) {
    driver.dispose()
    throw throwable
  }
  return driver
}

private class AndroidImeAnimationControlCompat(
  private val controller: WindowInsetsAnimationControllerCompat
) : AndroidImeAnimationControl {
  override val hiddenStateInsets: Insets
    get() = controller.hiddenStateInsets

  override val shownStateInsets: Insets
    get() = controller.shownStateInsets

  override val currentAlpha: Float
    get() = controller.currentAlpha

  override val isReady: Boolean
    get() = controller.isReady

  override fun setInsetsAndAlpha(insets: Insets, alpha: Float, fraction: Float) {
    controller.setInsetsAndAlpha(insets, alpha, fraction)
  }

  override fun finish(shown: Boolean) {
    controller.finish(shown)
  }
}
