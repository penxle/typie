package co.typie.ui.component.sheet

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.runtime.Immutable
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

enum class SheetMode {
  Modal,
  NonModalOverlay,
  Persistent,
}

enum class SheetDismissReason {
  Back,
  OutsideTap,
  Drag,
  Programmatic,
  Replaced,
}

sealed interface SheetResult<out R> {
  data class Completed<R>(
    val value: R,
  ) : SheetResult<R>

  data class Dismissed(
    val reason: SheetDismissReason,
  ) : SheetResult<Nothing>
}

fun <R> SheetResult<R>.completedOrNull(): R? =
  when (this) {
    is SheetResult.Completed -> value
    is SheetResult.Dismissed -> null
  }

@Immutable
data class SheetDismissPolicy(
  val back: Boolean = true,
  val outsideTap: Boolean = true,
  val dragDown: Boolean = true,
  val programmatic: Boolean = true,
) {
  fun allows(reason: SheetDismissReason): Boolean =
    when (reason) {
      SheetDismissReason.Back -> back
      SheetDismissReason.OutsideTap -> outsideTap
      SheetDismissReason.Drag -> dragDown
      SheetDismissReason.Programmatic,
      SheetDismissReason.Replaced -> programmatic
    }
}

sealed interface SheetInsetPolicy {
  data object Container : SheetInsetPolicy
  data object ContentTail : SheetInsetPolicy
  data object None : SheetInsetPolicy
}

@Immutable
data class SheetResolvedInset(
  val containerBottom: Dp = 0.dp,
  val contentTailBottom: Dp = 0.dp,
)

fun resolveSheetBottomInset(
  policy: SheetInsetPolicy,
  imeBottom: Dp,
  safeBottom: Dp,
): SheetResolvedInset {
  val bottom = maxOf(imeBottom, safeBottom)
  return when (policy) {
    SheetInsetPolicy.Container -> SheetResolvedInset(containerBottom = bottom)
    SheetInsetPolicy.ContentTail -> SheetResolvedInset(contentTailBottom = bottom)
    SheetInsetPolicy.None -> SheetResolvedInset()
  }
}

@Immutable
data class SheetPadding(
  val header: PaddingValues = PaddingValues(horizontal = 16.dp, vertical = 0.dp),
  val body: PaddingValues = PaddingValues(horizontal = 16.dp, vertical = 0.dp),
  val footer: PaddingValues = PaddingValues(horizontal = 16.dp, vertical = 0.dp),
) {
  companion object {
    val None = SheetPadding(
      header = PaddingValues(0.dp),
      body = PaddingValues(0.dp),
      footer = PaddingValues(0.dp),
    )
  }
}

sealed interface SheetHandleStyle {
  data object Hidden : SheetHandleStyle

  @Immutable
  data class Visible(
    val width: Dp = 36.dp,
    val height: Dp = 4.dp,
    val topPadding: Dp = 8.dp,
    val bottomPadding: Dp = 8.dp,
  ) : SheetHandleStyle
}

@Immutable
data class SheetScrim(
  val visible: Boolean = true,
  val opacity: Float = 0.4f,
  val blocksPointerInput: Boolean = true,
)

@Immutable
data class SheetChrome(
  val handle: SheetHandleStyle = SheetHandleStyle.Visible(),
  val scrim: SheetScrim = SheetScrim(),
  val topCornerRadius: Dp = 22.dp,
) {
  companion object {
    val Default = SheetChrome()
  }
}

sealed interface SheetExpansionPolicy {
  data object DragOrProgrammatic : SheetExpansionPolicy
  data object ProgrammaticOnly : SheetExpansionPolicy
}

sealed interface SheetCollapsePolicy {
  data object DragOrProgrammatic : SheetCollapsePolicy
  data object ProgrammaticOnly : SheetCollapsePolicy
}

enum class SheetDragDismissBehavior {
  FromMinDetent,
  FromCurrentDetent,
}

sealed interface SheetDetentId {
  data object Intrinsic : SheetDetentId

  @Immutable
  data class Fixed(
    val height: Dp,
  ) : SheetDetentId

  @Immutable
  data class Fraction(
    val fraction: Float,
  ) : SheetDetentId

  @Immutable
  data class TopGap(
    val gap: Dp,
  ) : SheetDetentId

  @Immutable
  data class Content(
    val maxTopGap: Dp?,
  ) : SheetDetentId

  @Immutable
  data class Custom(
    val value: String,
  ) : SheetDetentId
}

@Immutable
data class SheetDetentContext(
  val viewportHeight: Dp,
  val contentHeight: Dp,
)

sealed interface SheetDetent {
  val id: SheetDetentId

  data object Intrinsic : SheetDetent {
    override val id: SheetDetentId = SheetDetentId.Intrinsic
  }

  @Immutable
  data class Fixed(
    val height: Dp,
  ) : SheetDetent {
    override val id: SheetDetentId = SheetDetentId.Fixed(height)
  }

  @Immutable
  data class Fraction(
    val value: Float,
  ) : SheetDetent {
    override val id: SheetDetentId = SheetDetentId.Fraction(value)
  }

  @Immutable
  data class TopGap(
    val gap: Dp,
  ) : SheetDetent {
    override val id: SheetDetentId = SheetDetentId.TopGap(gap)
  }

  @Immutable
  data class Content(
    val maxTopGap: Dp? = null,
  ) : SheetDetent {
    override val id: SheetDetentId = SheetDetentId.Content(maxTopGap)
  }

  class Custom(
    override val id: SheetDetentId,
    val resolver: (SheetDetentContext) -> Dp,
  ) : SheetDetent
}

@Immutable
data class ResolvedSheetDetent(
  val id: SheetDetentId,
  val height: Dp,
)

sealed interface SheetSizePolicy {
  @Immutable
  data class Intrinsic(
    val topGap: Dp = SheetDefaults.IntrinsicTopGap,
  ) : SheetSizePolicy

  @Immutable
  data class Fixed(
    val height: Dp,
  ) : SheetSizePolicy

  @Immutable
  data class Max(
    val topGap: Dp,
  ) : SheetSizePolicy

  @Immutable
  data class Detents(
    val initial: SheetDetent,
    val available: List<SheetDetent>,
    val expansionPolicy: SheetExpansionPolicy = SheetExpansionPolicy.DragOrProgrammatic,
    val collapsePolicy: SheetCollapsePolicy = SheetCollapsePolicy.DragOrProgrammatic,
    val dragDismissBehavior: SheetDragDismissBehavior = SheetDragDismissBehavior.FromMinDetent,
  ) : SheetSizePolicy
}

@Immutable
data class SheetHapticPolicy(
  val onPresent: Boolean = true,
  val onDetentSnap: Boolean = false,
  val onDismiss: Boolean = false,
)

@Immutable
data class SheetOverlaySpec(
  val mode: SheetMode = SheetMode.Modal,
  val sizePolicy: SheetSizePolicy = SheetSizePolicy.Intrinsic(),
  val dismissPolicy: SheetDismissPolicy = SheetDismissPolicy(),
  val chrome: SheetChrome = SheetChrome.Default,
  val haptics: SheetHapticPolicy = SheetHapticPolicy(),
) {
  init {
    require(mode != SheetMode.Persistent) {
      "SheetOverlaySpec only supports overlay modes."
    }
  }
}

internal fun SheetSizePolicy.initialDetentId(): SheetDetentId =
  when (this) {
    is SheetSizePolicy.Intrinsic -> SheetDetentId.Intrinsic
    is SheetSizePolicy.Fixed -> SheetDetentId.Fixed(height)
    is SheetSizePolicy.Max -> SheetDetentId.TopGap(topGap)
    is SheetSizePolicy.Detents -> initial.id
  }

internal fun SheetSizePolicy.requiresContentMeasurement(): Boolean =
  when (this) {
    is SheetSizePolicy.Intrinsic -> true
    is SheetSizePolicy.Fixed,
    is SheetSizePolicy.Max,
    -> false

    is SheetSizePolicy.Detents -> (listOf(initial) + available)
      .any { it.requiresContentMeasurement() }
  }

internal fun SheetSizePolicy.allowsDragExpansion(): Boolean =
  when (this) {
    is SheetSizePolicy.Detents -> expansionPolicy == SheetExpansionPolicy.DragOrProgrammatic
    else -> true
  }

internal fun SheetSizePolicy.allowsDragCollapse(): Boolean =
  when (this) {
    is SheetSizePolicy.Detents -> collapsePolicy == SheetCollapsePolicy.DragOrProgrammatic
    else -> true
  }

internal fun SheetDetent.requiresContentMeasurement(): Boolean =
  when (this) {
    SheetDetent.Intrinsic -> true
    is SheetDetent.Content,
    is SheetDetent.Custom,
    -> true

    is SheetDetent.Fixed,
    is SheetDetent.Fraction,
    is SheetDetent.TopGap,
    -> false
  }
