package co.typie.ui.skeleton

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Paint
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.drawOutline
import androidx.compose.ui.graphics.drawscope.ContentDrawScope
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.node.CompositionLocalConsumerModifierNode
import androidx.compose.ui.node.DrawModifierNode
import androidx.compose.ui.node.GlobalPositionAwareModifierNode
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.node.currentValueOf
import androidx.compose.ui.node.invalidateDraw
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.unit.IntSize

class SkeletonUniteGroup(val shape: Shape) {
  internal val members = mutableStateListOf<UniteMember>()

  internal fun register(member: UniteMember) {
    members += member
  }

  internal fun unregister(member: UniteMember) {
    members -= member
  }

  internal fun getUnionBounds(relativeTo: LayoutCoordinates): Rect? {
    var union: Rect? = null
    for (member in members) {
      val coords = member.coords?.takeIf { it.isAttached } ?: continue
      val bounds = relativeTo.localBoundingBoxOf(coords, clipBounds = false)
      union =
        union?.let {
          Rect(
            left = minOf(it.left, bounds.left),
            top = minOf(it.top, bounds.top),
            right = maxOf(it.right, bounds.right),
            bottom = maxOf(it.bottom, bounds.bottom),
          )
        } ?: bounds
    }
    return union
  }
}

internal class UniteMember {
  var coords: LayoutCoordinates? by mutableStateOf(null)
}

private abstract class SkeletonNode :
  Modifier.Node(),
  DrawModifierNode,
  GlobalPositionAwareModifierNode,
  CompositionLocalConsumerModifierNode {

  private val member = UniteMember()
  private var registeredGroup: SkeletonUniteGroup? = null
  private val fadePaint = Paint()

  override fun onAttach() {
    syncGroupMembership()
  }

  override fun onDetach() {
    registeredGroup?.unregister(member)
    registeredGroup = null
  }

  override fun onGloballyPositioned(coordinates: LayoutCoordinates) {
    syncGroupMembership()
    member.coords = coordinates
  }

  private fun syncGroupMembership() {
    val current = currentValueOf(LocalSkeletonUnite)
    if (current !== registeredGroup) {
      registeredGroup?.unregister(member)
      registeredGroup = current
      registeredGroup?.register(member)
      invalidateDraw()
    }
  }

  final override fun ContentDrawScope.draw() {
    val state = currentValueOf(LocalSkeleton)
    val fraction = state.fraction.value

    when {
      fraction <= 0f -> drawContent()
      fraction < 1f -> {
        fadePaint.alpha = 1f - fraction
        drawContext.canvas.saveLayer(bounds = Rect(Offset.Zero, size), paint = fadePaint)
        drawContent()
        drawContext.canvas.restore()
      }
      else -> {
        // fraction >= 1f: children fully hidden — skip drawContent for perf
      }
    }

    if (fraction <= 0f) return

    val group = registeredGroup
    if (group != null) {
      drawUnionBone(state, fraction, group)
    } else {
      drawBone(state, fraction)
    }
  }

  private fun ContentDrawScope.drawUnionBone(
    state: SkeletonState,
    fraction: Float,
    group: SkeletonUniteGroup,
  ) {
    val coords = member.coords ?: return
    val union = group.getUnionBounds(coords) ?: return
    if (union.isEmpty) return
    val color = state.boneColor.value
    val effective = color.copy(alpha = color.alpha * fraction)
    val outline =
      group.shape.createOutline(
        size = Size(union.width, union.height),
        layoutDirection = layoutDirection,
        density = this,
      )
    translate(left = union.left, top = union.top) {
      drawOutline(outline = outline, color = effective)
    }
  }

  protected abstract fun ContentDrawScope.drawBone(state: SkeletonState, fraction: Float)
}

internal fun Modifier.skeletonBone(shape: Shape): Modifier = this then SkeletonBoneElement(shape)

private data class SkeletonBoneElement(val shape: Shape) : ModifierNodeElement<SkeletonBoneNode>() {
  override fun create(): SkeletonBoneNode = SkeletonBoneNode(shape)

  override fun update(node: SkeletonBoneNode) {
    node.shape = shape
  }
}

private class SkeletonBoneNode(var shape: Shape) : SkeletonNode() {
  override fun ContentDrawScope.drawBone(state: SkeletonState, fraction: Float) {
    val color = state.boneColor.value
    val outline = shape.createOutline(size, layoutDirection, this)
    drawOutline(outline = outline, color = color.copy(alpha = color.alpha * fraction))
  }
}

internal fun Modifier.skeletonTextBone(layoutResult: () -> TextLayoutResult?): Modifier =
  this then SkeletonTextBoneElement(layoutResult)

private data class SkeletonTextBoneElement(val layoutResult: () -> TextLayoutResult?) :
  ModifierNodeElement<SkeletonTextBoneNode>() {
  override fun create(): SkeletonTextBoneNode = SkeletonTextBoneNode(layoutResult)

  override fun update(node: SkeletonTextBoneNode) {
    node.layoutResult = layoutResult
  }
}

private class SkeletonTextBoneNode(var layoutResult: () -> TextLayoutResult?) : SkeletonNode() {
  override fun ContentDrawScope.drawBone(state: SkeletonState, fraction: Float) {
    val result = layoutResult() ?: return
    val color = state.boneColor.value
    val effective = color.copy(alpha = color.alpha * fraction)
    val cornerRadius = CornerRadius(4f * density)
    for (i in 0 until result.lineCount) {
      val left = result.getLineLeft(i)
      val top = result.getLineTop(i)
      val right = result.getLineRight(i)
      val bottom = result.getLineBottom(i)
      drawRoundRect(
        color = effective,
        topLeft = Offset(left, top),
        size = Size(right - left, bottom - top),
        cornerRadius = cornerRadius,
      )
    }
  }
}

internal fun Modifier.skeletonPointerIgnore(state: SkeletonState): Modifier =
  this then SkeletonPointerIgnoreElement(state)

private data class SkeletonPointerIgnoreElement(val state: SkeletonState) :
  ModifierNodeElement<SkeletonPointerIgnoreNode>() {
  override fun create(): SkeletonPointerIgnoreNode = SkeletonPointerIgnoreNode(state)

  override fun update(node: SkeletonPointerIgnoreNode) {
    node.state = state
  }
}

private class SkeletonPointerIgnoreNode(var state: SkeletonState) :
  Modifier.Node(), PointerInputModifierNode {

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) {
    if (state.fraction.value > 0.01f) {
      pointerEvent.changes.forEach { it.consume() }
    }
  }

  override fun onCancelPointerInput() = Unit
}
