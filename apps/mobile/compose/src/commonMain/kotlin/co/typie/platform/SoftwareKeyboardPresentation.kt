package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf

internal enum class SoftwareKeyboardPresentationEndpoint {
  Shown,
  Hidden,
}

internal enum class SoftwareKeyboardInteractionResolution {
  Shown,
  Hidden,
  Aborted,
}

internal data class SoftwareKeyboardInteractionState(
  val activeInteractionId: Long? = null,
  val hiddenProgress: Float = 0f,
  val resolutionVersion: Long = 0L,
  val lastResolvedInteractionId: Long? = null,
  val lastResolution: SoftwareKeyboardInteractionResolution? = null,
) {
  val unresolved: Boolean
    get() = activeInteractionId != null
}

internal interface SoftwareKeyboardPresentationDriver {
  fun updateHiddenProgress(progress: Float)

  fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit)

  fun dispose()
}

internal fun interface SoftwareKeyboardPresentationDriverFactory {
  fun acquire(onInvalidated: () -> Unit): SoftwareKeyboardPresentationDriver?
}

internal class SoftwareKeyboardPresentationController(
  private val driverFactory: SoftwareKeyboardPresentationDriverFactory
) {
  var interactionState by mutableStateOf(SoftwareKeyboardInteractionState())
    private set

  private var nextInteractionId = 0L
  private var nextContributionId = 0L
  private var acquisition: Acquisition? = null
  private var activeInteraction: ActiveInteraction? = null

  fun acquire(): SoftwareKeyboardPresentationSession? {
    activeInteraction?.let { interaction ->
      if (interaction.endpoint != null) return null
      return addContribution(interaction)
    }
    if (acquisition != null) return null

    val interactionId = ++nextInteractionId
    val pendingAcquisition = Acquisition(interactionId)
    acquisition = pendingAcquisition
    interactionState =
      interactionState.copy(activeInteractionId = interactionId, hiddenProgress = 0f)
    val driverResult = runCatching { driverFactory.acquire { invalidate(interactionId) } }
    acquisition = null
    val driver = driverResult.getOrNull()

    if (pendingAcquisition.invalidated || driverResult.isFailure) {
      interactionState =
        interactionState.resolved(
          interactionId = interactionId,
          resolution = SoftwareKeyboardInteractionResolution.Aborted,
        )
      if (driver != null) runCatching(driver::dispose)
      return null
    }
    if (driver == null) {
      interactionState = interactionState.copy(activeInteractionId = null, hiddenProgress = 0f)
      return null
    }

    val interaction = ActiveInteraction(id = interactionId, driver = driver)
    activeInteraction = interaction
    return addContribution(interaction)
  }

  fun dispose() {
    acquisition?.invalidated = true
    activeInteraction?.let { abort(it.id) }
  }

  internal fun updateHiddenProgress(interactionId: Long, contributionId: Long, progress: Float) {
    val interaction = activeContribution(interactionId, contributionId) ?: return
    val clampedProgress = progress.coerceIn(0f, 1f)
    val effectiveProgress =
      interaction.contributions.maxOf { (id, currentProgress) ->
        if (id == contributionId) clampedProgress else currentProgress
      }
    val updated =
      runCatching { interaction.driver.updateHiddenProgress(effectiveProgress) }.isSuccess
    if (!updated) {
      abort(interactionId)
      return
    }
    if (activeContribution(interactionId, contributionId) !== interaction) return
    interaction.contributions[contributionId] = clampedProgress
    interactionState = interactionState.copy(hiddenProgress = effectiveProgress)
  }

  internal fun finish(
    interactionId: Long,
    contributionId: Long,
    endpoint: SoftwareKeyboardPresentationEndpoint,
  ) {
    val interaction = activeContribution(interactionId, contributionId) ?: return
    if (
      endpoint == SoftwareKeyboardPresentationEndpoint.Shown && interaction.contributions.size > 1
    ) {
      removeContribution(interaction, contributionId)
      return
    }

    interaction.endpoint = endpoint
    interaction.contributions.clear()
    val requested =
      runCatching {
          interaction.driver.finish(endpoint) {
            accept(interactionId = interactionId, endpoint = endpoint)
          }
        }
        .isSuccess
    if (!requested) abort(interactionId)
  }

  internal fun dispose(interactionId: Long, contributionId: Long) {
    val interaction = activeContribution(interactionId, contributionId) ?: return
    if (interaction.contributions.size > 1) {
      removeContribution(interaction, contributionId)
    } else {
      abort(interactionId)
    }
  }

  private fun accept(interactionId: Long, endpoint: SoftwareKeyboardPresentationEndpoint) {
    val interaction =
      activeInteraction?.takeIf { it.id == interactionId && it.endpoint == endpoint } ?: return
    activeInteraction = null
    interactionState =
      interactionState.resolved(
        interactionId = interaction.id,
        resolution = endpoint.toResolution(),
      )
  }

  private fun abort(interactionId: Long) {
    val interaction = activeInteraction?.takeIf { it.id == interactionId } ?: return
    activeInteraction = null
    interactionState =
      interactionState.resolved(
        interactionId = interaction.id,
        resolution = SoftwareKeyboardInteractionResolution.Aborted,
      )
    runCatching(interaction.driver::dispose)
  }

  private fun invalidate(interactionId: Long) {
    val pendingAcquisition = acquisition
    if (pendingAcquisition?.id == interactionId) {
      pendingAcquisition.invalidated = true
      return
    }
    abort(interactionId)
  }

  private fun addContribution(interaction: ActiveInteraction): SoftwareKeyboardPresentationSession {
    val contributionId = ++nextContributionId
    interaction.contributions[contributionId] = 0f
    return SoftwareKeyboardPresentationSession(
      interactionId = interaction.id,
      contributionId = contributionId,
      controller = this,
    )
  }

  private fun activeContribution(interactionId: Long, contributionId: Long): ActiveInteraction? =
    activeInteraction?.takeIf {
      it.id == interactionId && it.endpoint == null && contributionId in it.contributions
    }

  private fun removeContribution(interaction: ActiveInteraction, contributionId: Long) {
    val effectiveProgress =
      interaction.contributions.asSequence().filter { it.key != contributionId }.maxOf { it.value }
    val updated =
      runCatching { interaction.driver.updateHiddenProgress(effectiveProgress) }.isSuccess
    if (!updated) {
      abort(interaction.id)
      return
    }
    if (activeContribution(interaction.id, contributionId) !== interaction) return
    interaction.contributions.remove(contributionId)
    interactionState = interactionState.copy(hiddenProgress = effectiveProgress)
  }

  private fun SoftwareKeyboardInteractionState.resolved(
    interactionId: Long,
    resolution: SoftwareKeyboardInteractionResolution,
  ): SoftwareKeyboardInteractionState =
    copy(
      activeInteractionId = null,
      resolutionVersion = resolutionVersion + 1,
      lastResolvedInteractionId = interactionId,
      lastResolution = resolution,
    )

  private class Acquisition(val id: Long, var invalidated: Boolean = false)

  private class ActiveInteraction(
    val id: Long,
    val driver: SoftwareKeyboardPresentationDriver,
    val contributions: MutableMap<Long, Float> = linkedMapOf(),
    var endpoint: SoftwareKeyboardPresentationEndpoint? = null,
  )

  companion object {
    val unavailable =
      SoftwareKeyboardPresentationController(SoftwareKeyboardPresentationDriverFactory { null })
  }
}

internal class SoftwareKeyboardPresentationSession
internal constructor(
  val interactionId: Long,
  private val contributionId: Long,
  private val controller: SoftwareKeyboardPresentationController,
) {
  fun updateHiddenProgress(progress: Float) {
    controller.updateHiddenProgress(
      interactionId = interactionId,
      contributionId = contributionId,
      progress = progress,
    )
  }

  fun finish(endpoint: SoftwareKeyboardPresentationEndpoint) {
    controller.finish(
      interactionId = interactionId,
      contributionId = contributionId,
      endpoint = endpoint,
    )
  }

  fun dispose() {
    controller.dispose(interactionId = interactionId, contributionId = contributionId)
  }
}

private fun SoftwareKeyboardPresentationEndpoint.toResolution() =
  when (this) {
    SoftwareKeyboardPresentationEndpoint.Shown -> SoftwareKeyboardInteractionResolution.Shown
    SoftwareKeyboardPresentationEndpoint.Hidden -> SoftwareKeyboardInteractionResolution.Hidden
  }

@Composable
internal expect fun rememberSoftwareKeyboardPresentationDriverFactory():
  SoftwareKeyboardPresentationDriverFactory

internal val LocalSoftwareKeyboardPresentationController = staticCompositionLocalOf {
  SoftwareKeyboardPresentationController.unavailable
}

@Composable
internal fun ProvideSoftwareKeyboardPresentation(content: @Composable () -> Unit) {
  val driverFactory = rememberSoftwareKeyboardPresentationDriverFactory()
  val controller = remember(driverFactory) { SoftwareKeyboardPresentationController(driverFactory) }
  DisposableEffect(controller) { onDispose(controller::dispose) }
  CompositionLocalProvider(
    LocalSoftwareKeyboardPresentationController provides controller,
    content = content,
  )
}
