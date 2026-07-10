package co.typie.screen.editor.editor.toolbar.bottom

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorTheme
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.ffi.BlockOp
import co.typie.editor.ffi.BlockquoteVariant
import co.typie.editor.ffi.Fragment
import co.typie.editor.ffi.HorizontalRuleVariant
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOp
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.screen.editor.editor.toolbar.BlockquoteVariantPanelTarget
import co.typie.screen.editor.editor.toolbar.HorizontalRuleVariantPanelTarget
import co.typie.screen.editor.editor.toolbar.TableBorderStylePanelTarget
import co.typie.screen.editor.editor.toolbar.ToolbarBottomPanelRadius
import co.typie.ui.component.Text
import co.typie.ui.component.scrollFog
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.min

@Composable
internal fun BottomToolbarHorizontalRuleVariants(
  target: HorizontalRuleVariantPanelTarget,
  onEditorMessage: (Message) -> Unit,
  onEditorInputRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  VariantList(modifier = modifier) {
    items(HorizontalRuleVariantItems, key = { it.name }) { variant ->
      VariantRow(
        label = variant.label,
        selected = variant == target.currentVariant,
        preview = { HorizontalRuleVariantPreview(variant = variant) },
        onClick = {
          target.messageOrNull(variant)?.let(onEditorMessage)
          onEditorInputRequest()
        },
      )
    }
  }
}

@Composable
internal fun BottomToolbarBlockquoteVariants(
  target: BlockquoteVariantPanelTarget,
  onEditorMessage: (Message) -> Unit,
  onEditorInputRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  VariantList(modifier = modifier) {
    items(BlockquoteVariantItems, key = { it.name }) { variant ->
      VariantRow(
        label = variant.label,
        selected = variant == target.currentVariant,
        preview = { BlockquoteVariantPreview(variant = variant) },
        onClick = {
          target.messageOrNull(variant)?.let(onEditorMessage)
          onEditorInputRequest()
        },
      )
    }
  }
}

@Composable
internal fun BottomToolbarTableBorderStyles(
  target: TableBorderStylePanelTarget,
  onEditorMessage: (Message) -> Unit,
  onEditorInputRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  VariantList(modifier = modifier) {
    items(TableBorderStyleItems, key = { it.name }) { style ->
      VariantRow(
        label = style.label,
        selected = style == target.currentStyle,
        preview = { TableBorderStylePreview(style = style) },
        previewWidth = TableBorderStylePreviewWidth,
        onClick = {
          target.messageOrNull(style)?.let(onEditorMessage)
          onEditorInputRequest()
        },
      )
    }
  }
}

@Composable
private fun VariantList(
  modifier: Modifier,
  content: androidx.compose.foundation.lazy.LazyListScope.() -> Unit,
) {
  val fogInsets = remember { PaddingValues(vertical = VariantPanelPadding) }
  LazyColumn(
    modifier =
      modifier.fillMaxSize().scrollFog(padding = fogInsets, color = AppTheme.colors.surfaceCanvas),
    contentPadding =
      PaddingValues(horizontal = VariantPanelPadding, vertical = VariantPanelPadding),
    verticalArrangement = Arrangement.spacedBy(4.dp),
    content = content,
  )
}

@Composable
private fun VariantRow(
  label: String,
  selected: Boolean,
  preview: @Composable () -> Unit,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
  previewWidth: Dp = VariantPreviewWidth,
) {
  val shape = VariantRowShape

  InteractionScope {
    val interactionSource =
      LocalInteractionSource.current ?: remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()
    val backgroundColor =
      when {
        selected || pressed -> AppTheme.colors.surfaceInset
        else -> Color.Transparent
      }

    Row(
      modifier =
        modifier
          .fillMaxWidth()
          .height(VariantRowHeight)
          .focusProperties { canFocus = false }
          .clip(shape)
          .background(backgroundColor, shape)
          .then(
            if (selected) Modifier.border(1.dp, AppTheme.colors.borderDefault, shape) else Modifier
          )
          .clickable(
            interactionSource = interactionSource,
            indication = null,
            role = Role.Button,
            onClickLabel = label,
            onClick = onClick,
          )
          .pressScale(0.985f)
          .padding(horizontal = 12.dp),
      horizontalArrangement = Arrangement.spacedBy(12.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Box(
        modifier = Modifier.width(previewWidth).height(VariantPreviewHeight),
        contentAlignment = Alignment.Center,
      ) {
        preview()
      }
      Text(
        text = label,
        style = AppTheme.typography.body,
        color = AppTheme.colors.textDefault,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier = Modifier.weight(1f),
      )
      if (selected) {
        Icon(
          icon = Lucide.Check,
          contentDescription = null,
          tint = AppTheme.colors.textDefault,
          modifier = Modifier.size(18.dp),
        )
      }
    }
  }
}

@Composable
private fun HorizontalRuleVariantPreview(variant: HorizontalRuleVariant) {
  val editorThemeColors = rememberEditorThemeColors()
  val color = editorThemeColors.getValue("ui.text.default")
  Canvas(Modifier.fillMaxWidth().height(VariantPreviewHeight)) {
    val y = size.height / 2f
    val lineHeight = 1.dp.toPx()
    val lineY = y - lineHeight / 2f
    val largeShapeSize = 10.dp.toPx()
    val smallShapeSize = 8.dp.toPx()

    when (variant) {
      HorizontalRuleVariant.Line -> {
        drawRect(color, topLeft = Offset(0f, lineY), size = Size(size.width, lineHeight))
      }
      HorizontalRuleVariant.DashedLine -> drawDashedLine(color, lineY, lineHeight)
      HorizontalRuleVariant.CircleLine -> {
        drawDecoratedLine(color = color, lineY = lineY, lineHeight = lineHeight)
        drawCircle(color, radius = largeShapeSize / 2f, center = Offset(size.width / 2f, y))
      }
      HorizontalRuleVariant.DiamondLine -> {
        drawDecoratedLine(color = color, lineY = lineY, lineHeight = lineHeight)
        drawDiamondStroke(
          color = color,
          center = Offset(size.width / 2f, y),
          radius = largeShapeSize / 2f,
          stroke = lineHeight,
        )
      }
      HorizontalRuleVariant.Circle -> {
        drawCircle(color, radius = largeShapeSize / 2f, center = Offset(size.width / 2f, y))
      }
      HorizontalRuleVariant.Diamond -> {
        drawDiamondStroke(
          color = color,
          center = Offset(size.width / 2f, y),
          radius = largeShapeSize / 2f,
          stroke = lineHeight,
        )
      }
      HorizontalRuleVariant.ThreeCircles -> {
        val radius = smallShapeSize / 2f
        val gap = smallShapeSize + 8.dp.toPx()
        repeat(3) { index ->
          drawCircle(
            color,
            radius = radius,
            center = Offset(size.width / 2f + (index - 1) * gap, y),
          )
        }
      }
      HorizontalRuleVariant.ThreeDiamonds -> {
        val radius = smallShapeSize / 2f
        val gap = smallShapeSize + 8.dp.toPx()
        repeat(3) { index ->
          drawDiamondStroke(
            color = color,
            center = Offset(size.width / 2f + (index - 1) * gap, y),
            radius = radius,
            stroke = lineHeight,
          )
        }
      }
      HorizontalRuleVariant.Zigzag -> drawZigzagLine(color, y, lineHeight)
    }
  }
}

@Composable
private fun BlockquoteVariantPreview(variant: BlockquoteVariant) {
  val editorThemeColors = rememberEditorThemeColors()
  val textMuted = editorThemeColors.getValue("ui.text.muted")
  val borderDefault = editorThemeColors.getValue("ui.border.default")
  val messageSent = editorThemeColors.getValue("ui.blockquote.message-sent")
  val messageReceived = editorThemeColors.getValue("ui.blockquote.message-received")

  if (variant == BlockquoteVariant.LeftQuote) {
    Box(
      Modifier.fillMaxWidth().height(VariantPreviewHeight),
      contentAlignment = Alignment.CenterStart,
    ) {
      Icon(
        icon = Typie.BlockquoteQuote,
        contentDescription = null,
        tint = textMuted,
        modifier = Modifier.size(16.dp),
      )
    }
    return
  }

  Canvas(Modifier.fillMaxWidth().height(VariantPreviewHeight)) {
    when (variant) {
      BlockquoteVariant.LeftLine -> {
        drawRect(
          color = borderDefault,
          topLeft = Offset.Zero,
          size = Size(width = 4.dp.toPx(), height = size.height),
        )
      }
      BlockquoteVariant.MessageSent -> {
        drawMessageBubble(color = messageSent, isSent = true)
      }
      BlockquoteVariant.MessageReceived -> {
        drawMessageBubble(color = messageReceived, isSent = false)
      }
      BlockquoteVariant.LeftQuote -> Unit
    }
  }
}

@Composable
private fun rememberEditorThemeColors(): Map<String, Color> {
  val themeVariant = currentEditorThemeVariant()
  return remember(themeVariant) { EditorTheme.resolve(themeVariant).colors }
}

private fun DrawScope.drawDashedLine(color: Color, y: Float, lineHeight: Float) {
  val segmentWidth = 16.dp.toPx()
  val dashWidth = segmentWidth * 0.5f
  var x = 0f
  while (x < size.width) {
    drawRect(
      color = color,
      topLeft = Offset(x, y),
      size = Size(width = min(dashWidth, size.width - x), height = lineHeight),
    )
    x += segmentWidth
  }
}

private fun DrawScope.drawDecoratedLine(color: Color, lineY: Float, lineHeight: Float) {
  val cx = size.width / 2f
  val containerHalf = size.width / 4f
  val shapeHalf = 5.dp.toPx() + 10.dp.toPx()
  val lineWidth = (containerHalf - shapeHalf).coerceAtLeast(0f)
  drawRect(
    color = color,
    topLeft = Offset(cx - containerHalf, lineY),
    size = Size(width = lineWidth, height = lineHeight),
  )
  drawRect(
    color = color,
    topLeft = Offset(cx + shapeHalf, lineY),
    size = Size(width = lineWidth, height = lineHeight),
  )
}

private fun DrawScope.drawMessageBubble(color: Color, isSent: Boolean) {
  val tailSize = 10.dp.toPx()
  val tailReach = tailSize * 0.4f
  val tailOverflow = tailSize * 0.15f
  val bubbleWidth = size.width * 0.8f
  val bubbleHeight = size.height * 0.8f - tailOverflow
  val left = if (isSent) size.width - bubbleWidth - tailReach else tailReach
  val top = (size.height - bubbleHeight - tailOverflow) / 2f
  val rect = Rect(offset = Offset(left, top), size = Size(bubbleWidth, bubbleHeight))
  val radius = 18.dp.toPx().coerceAtMost(bubbleHeight / 2f)
  val rounded = CornerRadius(radius, radius)
  val square = CornerRadius(0f, 0f)
  val bubblePath =
    Path().apply {
      addRoundRect(
        RoundRect(
          rect = rect,
          topLeft = rounded,
          topRight = rounded,
          bottomRight = if (isSent) square else rounded,
          bottomLeft = if (isSent) rounded else square,
        )
      )
    }
  drawPath(path = bubblePath, color = color)
  drawPath(
    path =
      buildMessageTailPath(
        left = left,
        top = top,
        width = bubbleWidth,
        height = bubbleHeight,
        size = tailSize,
        isSent = isSent,
      ),
    color = color,
  )
}

private fun buildMessageTailPath(
  left: Float,
  top: Float,
  width: Float,
  height: Float,
  size: Float,
  isSent: Boolean,
): Path =
  Path().apply {
    if (isSent) {
      moveTo(left + width - size * 0.8f, top + height)
      quadraticTo(left + width, top + height, left + width, top + height - size * 0.5f)
      quadraticTo(
        left + width,
        top + height,
        left + width + size * 0.4f,
        top + height + size * 0.15f,
      )
      quadraticTo(
        left + width - size * 0.2f,
        top + height + size * 0.05f,
        left + width - size * 0.8f,
        top + height,
      )
    } else {
      moveTo(left + size * 0.8f, top + height)
      quadraticTo(left, top + height, left, top + height - size * 0.5f)
      quadraticTo(left, top + height, left - size * 0.4f, top + height + size * 0.15f)
      quadraticTo(left + size * 0.2f, top + height + size * 0.05f, left + size * 0.8f, top + height)
    }
    close()
  }

private fun DrawScope.drawZigzagLine(color: Color, y: Float, stroke: Float) {
  val path = Path()
  val points = 8
  val segmentWidth = 8.dp.toPx()
  val totalWidth = (points - 1) * segmentWidth
  val startX = size.width / 2f - totalWidth / 2f
  val amplitude = 4.dp.toPx()
  for (i in 0 until points) {
    val x = startX + i * segmentWidth
    val pointY = if (i % 2 == 0) y + amplitude else y - amplitude
    if (i == 0) {
      path.moveTo(x, pointY)
    } else {
      path.lineTo(x, pointY)
    }
  }
  drawPath(
    path = path,
    color = color,
    style = Stroke(width = stroke, cap = StrokeCap.Round, join = StrokeJoin.Round),
  )
}

private fun DrawScope.drawDiamondStroke(
  color: Color,
  center: Offset,
  radius: Float,
  stroke: Float,
) {
  val path =
    Path().apply {
      moveTo(center.x, center.y - radius)
      lineTo(center.x + radius, center.y)
      lineTo(center.x, center.y + radius)
      lineTo(center.x - radius, center.y)
      close()
    }
  drawPath(path = path, color = color, style = Stroke(width = stroke))
}

@Composable
private fun TableBorderStylePreview(style: TableBorderStyle) {
  val editorThemeColors = rememberEditorThemeColors()
  val color = editorThemeColors.getValue("ui.text.default")

  if (style == TableBorderStyle.None) {
    Icon(
      icon = Lucide.Ban,
      contentDescription = null,
      tint = color,
      modifier = Modifier.size(TableBorderStyleIconSize),
    )
    return
  }

  Canvas(Modifier.size(TableBorderStyleIconSize)) {
    val stroke = 2.dp.toPx()
    val y = size.height / 2f
    when (style) {
      TableBorderStyle.Solid -> drawLine(color, Offset(0f, y), Offset(size.width, y), stroke)
      TableBorderStyle.Dashed ->
        drawTableBorderStyleDashedLine(color = color, y = y, stroke = stroke)
      TableBorderStyle.Dotted ->
        drawTableBorderStyleDottedLine(color = color, y = y, stroke = stroke)
      TableBorderStyle.None -> Unit
    }
  }
}

private fun DrawScope.drawTableBorderStyleDashedLine(color: Color, y: Float, stroke: Float) {
  val dashPx = 4.dp.toPx()
  val gapPx = 2.dp.toPx()
  repeat(3) { index ->
    val x = index * (dashPx + gapPx)
    drawLine(color, Offset(x, y), Offset(x + dashPx, y), stroke)
  }
}

private fun DrawScope.drawTableBorderStyleDottedLine(color: Color, y: Float, stroke: Float) {
  val radius = stroke / 2f
  val gap = size.width / 3f
  repeat(3) { index ->
    drawCircle(color = color, radius = radius, center = Offset(x = gap * index + gap / 2f, y = y))
  }
}

internal fun HorizontalRuleVariantPanelTarget.messageOrNull(
  variant: HorizontalRuleVariant
): Message? = if (variant == currentVariant) null else message(variant)

private fun HorizontalRuleVariantPanelTarget.message(variant: HorizontalRuleVariant): Message =
  when (this) {
    HorizontalRuleVariantPanelTarget.Insertion ->
      fragmentInsertion(PlainNode.HorizontalRule(variant = variant))
    is HorizontalRuleVariantPanelTarget.Existing ->
      Message.Node(
        NodeOp.SetAttrs(id = nodeId, attrs = PlainNode.HorizontalRule(variant = variant))
      )
  }

internal fun BlockquoteVariantPanelTarget.messageOrNull(variant: BlockquoteVariant): Message? =
  if (variant == currentVariant) null else message(variant)

private fun BlockquoteVariantPanelTarget.message(variant: BlockquoteVariant): Message =
  when (this) {
    BlockquoteVariantPanelTarget.Selection ->
      Message.Block(BlockOp.ToggleBlockquote(variant = variant))
    is BlockquoteVariantPanelTarget.Existing ->
      Message.Node(NodeOp.SetAttrs(id = nodeId, attrs = PlainNode.Blockquote(variant = variant)))
  }

internal fun TableBorderStylePanelTarget.messageOrNull(style: TableBorderStyle): Message? =
  if (style == currentStyle) {
    null
  } else {
    Message.Node(NodeOp.Table(id = tableId, op = TableOp.SetBorderStyle(borderStyle = style)))
  }

private fun fragmentInsertion(node: PlainNode): Message.Insertion =
  Message.Insertion(InsertionOp.Fragment(Fragment(node = node)))

private val HorizontalRuleVariantItems =
  listOf(
    HorizontalRuleVariant.Line,
    HorizontalRuleVariant.DashedLine,
    HorizontalRuleVariant.CircleLine,
    HorizontalRuleVariant.DiamondLine,
    HorizontalRuleVariant.Circle,
    HorizontalRuleVariant.Diamond,
    HorizontalRuleVariant.ThreeCircles,
    HorizontalRuleVariant.ThreeDiamonds,
    HorizontalRuleVariant.Zigzag,
  )

private val BlockquoteVariantItems =
  listOf(
    BlockquoteVariant.LeftLine,
    BlockquoteVariant.LeftQuote,
    BlockquoteVariant.MessageSent,
    BlockquoteVariant.MessageReceived,
  )

private val TableBorderStyleItems =
  listOf(
    TableBorderStyle.Solid,
    TableBorderStyle.Dashed,
    TableBorderStyle.Dotted,
    TableBorderStyle.None,
  )

private val HorizontalRuleVariant.label: String
  get() =
    when (this) {
      HorizontalRuleVariant.Line -> "실선"
      HorizontalRuleVariant.DashedLine -> "점선"
      HorizontalRuleVariant.CircleLine -> "원 장식 선"
      HorizontalRuleVariant.DiamondLine -> "마름모 장식 선"
      HorizontalRuleVariant.Circle -> "원"
      HorizontalRuleVariant.Diamond -> "마름모"
      HorizontalRuleVariant.ThreeCircles -> "원 3개"
      HorizontalRuleVariant.ThreeDiamonds -> "마름모 3개"
      HorizontalRuleVariant.Zigzag -> "지그재그"
    }

private val BlockquoteVariant.label: String
  get() =
    when (this) {
      BlockquoteVariant.LeftLine -> "왼쪽 선"
      BlockquoteVariant.LeftQuote -> "왼쪽 따옴표"
      BlockquoteVariant.MessageSent -> "보낸 메시지"
      BlockquoteVariant.MessageReceived -> "받은 메시지"
    }

private val VariantPanelPadding = 12.dp
private val TableBorderStyle.label: String
  get() =
    when (this) {
      TableBorderStyle.Solid -> "실선"
      TableBorderStyle.Dashed -> "파선"
      TableBorderStyle.Dotted -> "점선"
      TableBorderStyle.None -> "없음"
    }

private val VariantRowHeight = 52.dp
private val VariantPreviewWidth = 112.dp
private val VariantPreviewHeight = 24.dp
private val TableBorderStylePreviewWidth = 20.dp
private val TableBorderStyleIconSize = 16.dp
private val VariantRowShape =
  AppShapes.rounded(maxOf(AppShapes.sm, ToolbarBottomPanelRadius - VariantPanelPadding))
