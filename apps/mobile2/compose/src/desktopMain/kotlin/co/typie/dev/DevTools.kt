package co.typie.dev

import androidx.compose.runtime.snapshotFlow
import java.awt.Color
import java.awt.Cursor
import java.awt.Dimension
import java.awt.FlowLayout
import java.awt.Font
import java.awt.Graphics
import java.awt.Graphics2D
import java.awt.RenderingHints
import java.awt.Window
import java.awt.event.ComponentAdapter
import java.awt.event.ComponentEvent
import java.awt.event.MouseAdapter
import java.awt.event.MouseEvent
import java.awt.geom.Ellipse2D
import java.awt.geom.RoundRectangle2D
import java.util.prefs.Preferences
import javax.swing.BorderFactory
import javax.swing.BoxLayout
import javax.swing.JPanel
import javax.swing.JWindow
import javax.swing.SwingUtilities
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.launchIn
import kotlinx.coroutines.flow.onEach

private val PanelBackground = Color(0x2C, 0x2C, 0x2E)
private val ItemHover = Color(0x3A, 0x3A, 0x3C)
private val TextPrimary = Color(0xFF, 0xFF, 0xFF)
private val TextSecondary = Color(0x98, 0x98, 0x9D)
private val AccentMuted = Color(0x8E, 0x8E, 0x93)
private val AccentSuccess = Color(0x30, 0xD1, 0x58)
private val AccentWarning = Color(0xFF, 0x9F, 0x0A)
private val AccentDanger = Color(0xFF, 0x45, 0x3A)
private val AccentInfo = Color(0x0A, 0x84, 0xFF)
private val AccentHighlight = Color(0xBF, 0x5A, 0xF2)

private fun DevToolsAccent.accentColor(): Color =
  when (this) {
    DevToolsAccent.Muted -> AccentMuted
    DevToolsAccent.Success -> AccentSuccess
    DevToolsAccent.Warning -> AccentWarning
    DevToolsAccent.Danger -> AccentDanger
    DevToolsAccent.Info -> AccentInfo
    DevToolsAccent.Highlight -> AccentHighlight
  }

fun createDevToolsWindow(mainWindow: Window, networkSimulator: NetworkSimulator): JWindow {
  val devWindow = JWindow()
  devWindow.isAlwaysOnTop = true
  devWindow.background = Color(0, 0, 0, 0)

  val scope = CoroutineScope(Dispatchers.Main)

  // Icon button
  val iconButton =
    object : JPanel() {
      init {
        isOpaque = false
        preferredSize = Dimension(32, 32)
        cursor = Cursor.getPredefinedCursor(Cursor.HAND_CURSOR)
      }

      override fun paintComponent(g: Graphics) {
        val g2 = g as Graphics2D
        g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
        // Circle background
        g2.color = PanelBackground
        g2.fill(Ellipse2D.Double(0.0, 0.0, 32.0, 32.0))

        val dotSize = 6.0
        val dotGap = 2.0
        val accents = devToolsCollapsedIndicatorAccents(networkPreset = networkSimulator.preset)
        val totalHeight = accents.size * dotSize + (accents.size - 1) * dotGap
        val x = (32.0 - dotSize) / 2
        var y = (32.0 - totalHeight) / 2

        accents.forEach { accent ->
          g2.color = accent.accentColor()
          g2.fill(Ellipse2D.Double(x, y, dotSize, dotSize))
          y += dotSize + dotGap
        }
      }
    }

  // Dropdown panel
  val dropdownPanel =
    object : JPanel() {
      init {
        isOpaque = false
        layout = BoxLayout(this, BoxLayout.Y_AXIS)
        border = BorderFactory.createEmptyBorder(4, 4, 4, 4)
        isVisible = false
      }

      override fun paintComponent(g: Graphics) {
        val g2 = g as Graphics2D
        g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
        g2.color = PanelBackground
        g2.fill(RoundRectangle2D.Double(0.0, 0.0, width.toDouble(), height.toDouble(), 16.0, 16.0))
      }
    }

  val itemFont = Font("SF Pro Text", Font.PLAIN, 12)
  val itemFontBold = Font("SF Pro Text", Font.BOLD, 12)

  fun createSectionLabel(text: String): JPanel {
    return object : JPanel() {
      init {
        isOpaque = false
        maximumSize = Dimension(Int.MAX_VALUE, 20)
        preferredSize = Dimension(132, 20)
      }

      override fun paintComponent(g: Graphics) {
        val g2 = g as Graphics2D
        g2.setRenderingHint(
          RenderingHints.KEY_TEXT_ANTIALIASING,
          RenderingHints.VALUE_TEXT_ANTIALIAS_ON,
        )
        g2.color = TextSecondary
        g2.font = itemFontBold
        g2.drawString(text, 8, 14)
      }
    }
  }

  fun createOptionItem(
    labelText: String,
    accentColor: Color,
    selected: () -> Boolean,
    onClick: () -> Unit,
  ): JPanel {
    val item =
      object : JPanel(FlowLayout(FlowLayout.LEFT, 8, 4)) {
        init {
          isOpaque = false
          cursor = Cursor.getPredefinedCursor(Cursor.HAND_CURSOR)
          maximumSize = Dimension(Int.MAX_VALUE, 28)
        }

        override fun paintComponent(g: Graphics) {
          val g2 = g as Graphics2D
          g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
          if (selected()) {
            g2.color = ItemHover
            g2.fill(
              RoundRectangle2D.Double(0.0, 0.0, width.toDouble(), height.toDouble(), 12.0, 12.0)
            )
          }
        }
      }

    val dot =
      object : JPanel() {
        init {
          isOpaque = false
          preferredSize = Dimension(8, 8)
        }

        override fun paintComponent(g: Graphics) {
          val g2 = g as Graphics2D
          g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
          g2.color = accentColor
          g2.fill(Ellipse2D.Double(0.0, 0.0, 8.0, 8.0))
        }
      }

    val label =
      object : JPanel() {
        init {
          isOpaque = false
          preferredSize = Dimension(104, 16)
        }

        override fun paintComponent(g: Graphics) {
          val g2 = g as Graphics2D
          g2.setRenderingHint(
            RenderingHints.KEY_TEXT_ANTIALIASING,
            RenderingHints.VALUE_TEXT_ANTIALIAS_ON,
          )
          val isSelected = selected()
          g2.color = if (isSelected) TextPrimary else TextSecondary
          g2.font = if (isSelected) itemFontBold else itemFont
          g2.drawString(labelText, 0, 12)
        }
      }

    item.add(dot)
    item.add(label)

    item.addMouseListener(
      object : MouseAdapter() {
        override fun mouseClicked(e: MouseEvent) {
          onClick()
          dropdownPanel.isVisible = false
          devWindow.pack()
        }
      }
    )

    return item
  }

  dropdownPanel.add(createSectionLabel("Network"))

  NetworkPreset.entries.forEach { option ->
    dropdownPanel.add(
      createOptionItem(
        labelText = option.name,
        accentColor = option.devToolsAccent().accentColor(),
        selected = { networkSimulator.preset == option },
        onClick = {
          networkSimulator.select(option)
          Preferences.userRoot().node("co/typie").put("networkPreset", option.name)
        },
      )
    )
  }

  dropdownPanel.add(createSectionLabel("Bootstrap"))

  // Toggle dropdown on icon click
  iconButton.addMouseListener(
    object : MouseAdapter() {
      override fun mouseClicked(e: MouseEvent) {
        dropdownPanel.isVisible = !dropdownPanel.isVisible
        devWindow.pack()
      }
    }
  )

  // Content panel
  val contentPanel =
    JPanel().apply {
      isOpaque = false
      layout = BoxLayout(this, BoxLayout.Y_AXIS)
      add(iconButton)
      add(dropdownPanel)
    }

  devWindow.contentPane = contentPanel

  // Position sync
  fun syncPosition() {
    devWindow.setLocation(mainWindow.x + mainWindow.width + 8, mainWindow.y)
  }

  mainWindow.addComponentListener(
    object : ComponentAdapter() {
      override fun componentMoved(e: ComponentEvent) = syncPosition()

      override fun componentResized(e: ComponentEvent) = syncPosition()
    }
  )

  // Repaint on preset change
  snapshotFlow { networkSimulator.preset }
    .onEach { SwingUtilities.invokeLater { devWindow.repaint() } }
    .launchIn(scope)

  devWindow.pack()
  syncPosition()
  devWindow.isVisible = true

  return devWindow
}
