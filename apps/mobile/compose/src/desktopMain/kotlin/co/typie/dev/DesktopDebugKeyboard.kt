package co.typie.dev

// cspell:ignore smol floof awoo

import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.hoverable
import androidx.compose.foundation.interaction.HoverInteraction
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.union
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import co.typie.ext.LocalInteractionSource
import co.typie.ext.TextInputClient
import co.typie.ext.TextInputKey
import co.typie.ext.pressScale
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import java.awt.Component
import java.awt.KeyboardFocusManager
import java.awt.event.KeyEvent
import javax.swing.SwingUtilities
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.swing.Swing

private const val DesktopDebugKeyboardHideDelayMillis = 120L
private const val DesktopDebugKeyboardAnimationDurationMillis = 220
private const val DesktopDebugKeyboardRepeatDelayMillis = 360L
private const val DesktopDebugKeyboardRepeatIntervalMillis = 44L
private val DesktopDebugKeyboardDefaultHeight = 278.dp
private val DesktopDebugKeyboardBackground = Color(0xFFE7EBF5)
private val DesktopDebugKeyboardBody = Color(0xFFF4F7FC)
private val DesktopDebugKeyboardKey = Color(0xFFFFFFFF)
private val DesktopDebugKeyboardMint = Color(0xFFDDF6EC)
private val DesktopDebugKeyboardPeach = Color(0xFFFFE8D9)
private val DesktopDebugKeyboardLavender = Color(0xFFE9E3FF)
private val DesktopDebugKeyboardYellow = Color(0xFFFFF0BE)
private val DesktopDebugKeyboardBlue = Color(0xFFDDEBFF)
private val DesktopDebugKeyboardPink = Color(0xFFFFE1EC)
private val DesktopDebugKeyboardHover = Color(0xFFFFFFFF).copy(alpha = 0.32f)
private val DesktopDebugKeyboardWordStream =
  listOf(
    "wow ",
    "such soft ",
    "very cozy ",
    "much smol ",
    "so floof ",
    "gentle paws ",
    "tiny awoo ",
    "warm bean ",
  )
private const val DesktopDebugKeyboardCharStream = "such smol very wow so soft "
private const val DesktopDebugKeyboardDigitStream = "0123456789"
private val DesktopDebugKeyboardJapaneseFixtures =
  listOf(
    listOf("き", "きゅ", "きゅん", "きゅんっ", "きゅんっ ", "きゅんっ ぴ", "きゅんっ ぴょ", "きゅんっ ぴょん", "きゅんっ ぴょん♡"),
    listOf("き", "きら", "きらっ", "きらっ☆", "きらっ☆ ", "きらっ☆ る", "きらっ☆ るん", "きらっ☆ るんっ", "きらっ☆ るんっ☆"),
    listOf(
      "き",
      "きょ",
      "きょう",
      "きょうも",
      "きょうも ",
      "きょうも だ",
      "きょうも だい",
      "きょうも だいす",
      "きょうも だいすき",
      "今日も だいすき",
      "今日も 大好き",
      "今日も 大好き♡",
    ),
    listOf(
      "お",
      "おふ",
      "おふと",
      "おふとん",
      "おふとん ",
      "おふとん さ",
      "おふとん さい",
      "おふとん さいこ",
      "おふとん さいこう",
      "お布団 さいこう",
      "お布団 最高",
      "お布団 最高♡",
    ),
  )
private val DesktopDebugKeyboardKoreanFixtures =
  listOf(
    listOf(
      "ㅇ",
      "아",
      "앙",
      "앙ㄴ",
      "앙녀",
      "앙녕",
      "앙녀ㅇ",
      "앙녀어",
      "앙녀엉",
      "앙녀엉 ",
      "앙녀엉 ㄲ",
      "앙녀엉 뀨",
      "앙녀엉 뀨♡",
    ),
    listOf("ㅁ", "마", "말", "말ㄹ", "말라", "말랑", "말랑ㅋ", "말랑코", "말랑콩", "말랑콩ㄸ", "말랑콩떠", "말랑콩떡", "말랑콩떡♡"),
    listOf("ㅉ", "쪼", "쫀", "쫀ㄷ", "쫀드", "쫀득", "쫀득ㅈ", "쫀득제", "쫀득젤", "쫀득젤ㄹ", "쫀득젤리", "쫀득젤리♡"),
  )

internal object DesktopDebugKeyboard {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Swing)
  private val focusedOwners = linkedSetOf<Any>()
  private val clients = mutableStateMapOf<Any, TextInputClient>()
  private var hideJob: Job? = null
  private var clearRecentClientJob: Job? = null

  private var isVisible by mutableStateOf(false)
  private var activeOwner: Any? by mutableStateOf(null)
  private var recentClient: TextInputClient? by mutableStateOf(null)
  private var isPointerInteracting by mutableStateOf(false)
  private var charStreamIndex by mutableStateOf(0)
  private var wordStreamIndex by mutableStateOf(0)
  private var digitStreamIndex by mutableStateOf(0)
  private var japaneseFixtureBankIndex by mutableStateOf(0)
  private var japaneseFixtureStepIndex by mutableStateOf(0)
  private var koreanFixtureBankIndex by mutableStateOf(0)
  private var koreanFixtureStepIndex by mutableStateOf(0)
  private var activeFixtureLanguage: Boolean? by mutableStateOf(null)

  var hardwareKeyboardConnected by mutableStateOf(false)
    private set

  var height by mutableStateOf(DesktopDebugKeyboardDefaultHeight)
    private set

  val visible: Boolean
    get() = isVisible

  fun notifyFocusChanged(owner: Any, isFocused: Boolean) {
    if (isFocused) {
      val focusGained = owner !in focusedOwners
      focusedOwners.remove(owner)
      focusedOwners.add(owner)
      hideJob?.cancel()
      if (clients.containsKey(owner)) {
        activeOwner = owner
        updateRecentClient(clients.getValue(owner))
      }
      if (focusGained && !hardwareKeyboardConnected) {
        isVisible = true
      }
      return
    }

    focusedOwners.remove(owner)
    if (activeOwner == owner) {
      activeOwner = focusedOwners.lastOrNull { clients.containsKey(it) }
    }
    if (focusedOwners.isNotEmpty()) return
    if (isPointerInteracting) return

    scheduleHideIfNeeded()
  }

  fun updateHardwareKeyboardConnected(connected: Boolean) {
    hardwareKeyboardConnected = connected
    if (connected) {
      hideKeyboardSurface()
    } else {
      showKeyboard()
    }
  }

  fun registerClient(owner: Any, client: TextInputClient?) {
    if (client == null) {
      val removed = clients.remove(owner)
      if (activeOwner == owner) {
        activeOwner = focusedOwners.lastOrNull { clients.containsKey(it) }
      }
      if (removed != null) {
        keepRecentClientBriefly(removed)
      }
      return
    }

    clients[owner] = client
    if (owner in focusedOwners) {
      activeOwner = owner
      updateRecentClient(client)
    }
  }

  @Composable
  fun asWindowInsets(baseInsets: WindowInsets): WindowInsets {
    val animatedHeight = animatedHeight()
    return baseInsets.union(WindowInsets(bottom = animatedHeight))
  }

  @Composable
  fun Overlay(modifier: Modifier = Modifier) {
    val animatedHeight = animatedHeight()
    if (animatedHeight <= 0.dp) return

    Box(
      modifier
        .fillMaxWidth()
        .height(animatedHeight)
        .pointerInput(Unit) {
          awaitEachGesture {
            awaitFirstDown(requireUnconsumed = false)
            beginPointerInteraction()
            waitForUpOrCancellation()
            endPointerInteraction()
          }
        }
        .background(DesktopDebugKeyboardBackground.copy(alpha = 0.98f))
        .border(1.dp, AppTheme.colors.borderHairline.copy(alpha = 0.95f))
        .padding(horizontal = 10.dp, vertical = 10.dp)
    ) {
      Column(
        modifier =
          Modifier.fillMaxWidth()
            .background(DesktopDebugKeyboardBody.copy(alpha = 0.92f), RoundedCornerShape(18.dp))
            .padding(10.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        KeyboardRow(
          listOf(
            ActionKeySpec("jp きゅん☆", DesktopDebugKeyboardLavender, repeatable = true) {
              advanceCompositionFixture(DesktopDebugKeyboardJapaneseFixtures, isJapanese = true)
            },
            ActionKeySpec("kr 뀨♡", DesktopDebugKeyboardPink, repeatable = true) {
              advanceCompositionFixture(DesktopDebugKeyboardKoreanFixtures, isJapanese = false)
            },
            ActionKeySpec("commit", DesktopDebugKeyboardYellow) {
              resetFixtureProgress()
              withRefocusedClient { it.finishComposition() }
            },
          )
        )

        KeyboardRow(
          listOf(
            ActionKeySpec("such smol", DesktopDebugKeyboardMint, repeatable = true) {
              typeNextChar()
            },
            ActionKeySpec("much wow", DesktopDebugKeyboardPeach) { typeNextWord() },
            ActionKeySpec("123", DesktopDebugKeyboardBlue, repeatable = true) { typeNextDigit() },
            ActionKeySpec("space", DesktopDebugKeyboardKey, 1.8f, repeatable = true) {
              dispatchTypedChar(' ')
            },
            ActionKeySpec("enter", DesktopDebugKeyboardYellow, 1.2f) {
              dispatchSpecialKey(KeyEvent.VK_ENTER, TextInputKey.Enter)
            },
            ActionKeySpec("⌫", DesktopDebugKeyboardKey, 1.1f, repeatable = true) {
              dispatchSpecialKey(KeyEvent.VK_BACK_SPACE, TextInputKey.Backspace)
            },
          )
        )

        KeyboardRow(
          listOf(
            ActionKeySpec("a", DesktopDebugKeyboardKey, repeatable = true) {
              dispatchTypedChar('a')
            },
            ActionKeySpec("e", DesktopDebugKeyboardKey, repeatable = true) {
              dispatchTypedChar('e')
            },
            ActionKeySpec("i", DesktopDebugKeyboardKey, repeatable = true) {
              dispatchTypedChar('i')
            },
            ActionKeySpec("o", DesktopDebugKeyboardKey, repeatable = true) {
              dispatchTypedChar('o')
            },
            ActionKeySpec("u", DesktopDebugKeyboardKey, repeatable = true) {
              dispatchTypedChar('u')
            },
            ActionKeySpec("1", DesktopDebugKeyboardKey, repeatable = true) {
              dispatchTypedChar('1')
            },
            ActionKeySpec("0", DesktopDebugKeyboardKey, repeatable = true) {
              dispatchTypedChar('0')
            },
          )
        )

        Row(
          modifier = Modifier.fillMaxWidth(),
          horizontalArrangement = Arrangement.End,
          verticalAlignment = Alignment.CenterVertically,
        ) {
          KeyboardKey(
            spec = ActionKeySpec("hide", DesktopDebugKeyboardPink, 1f) { dismissInput() },
            modifier = Modifier.size(width = 86.dp, height = 34.dp),
          )
        }
      }
    }
  }

  internal fun hideKeyboardSurface() {
    resetFixtureProgress()
    hideJob?.cancel()
    isVisible = false
  }

  internal fun dismissInput() {
    rememberedClient()?.dismiss()
    clearRecentClient()
    hideKeyboardSurface()
  }

  internal fun showKeyboard() {
    if (hardwareKeyboardConnected) return
    val owner = focusedOwners.lastOrNull { clients.containsKey(it) } ?: return
    val client = clients[owner] ?: return
    hideJob?.cancel()
    activeOwner = owner
    updateRecentClient(client)
    isVisible = true
  }

  private fun beginPointerInteraction() {
    isPointerInteracting = true
    hideJob?.cancel()
    if (!hardwareKeyboardConnected) {
      isVisible = true
    }
  }

  private fun endPointerInteraction() {
    isPointerInteracting = false
    scheduleHideIfNeeded()
  }

  private fun typeNextChar() {
    val next = DesktopDebugKeyboardCharStream[charStreamIndex]
    charStreamIndex = (charStreamIndex + 1) % DesktopDebugKeyboardCharStream.length
    dispatchTypedChar(next)
  }

  private fun typeNextWord() {
    val next = DesktopDebugKeyboardWordStream[wordStreamIndex]
    wordStreamIndex = (wordStreamIndex + 1) % DesktopDebugKeyboardWordStream.size
    dispatchText(next)
  }

  private fun typeNextDigit() {
    val next = DesktopDebugKeyboardDigitStream[digitStreamIndex]
    digitStreamIndex = (digitStreamIndex + 1) % DesktopDebugKeyboardDigitStream.length
    dispatchTypedChar(next)
  }

  private fun advanceCompositionFixture(banks: List<List<String>>, isJapanese: Boolean) {
    withRefocusedClient { client ->
      if (activeFixtureLanguage != null && activeFixtureLanguage != isJapanese) {
        resetFixtureProgress(activeFixtureLanguage)
      }
      if (activeFixtureLanguage == isJapanese && !client.hasActiveComposition) {
        resetFixtureProgress(isJapanese)
      }

      val bankIndex = if (isJapanese) japaneseFixtureBankIndex else koreanFixtureBankIndex
      val stepIndex = if (isJapanese) japaneseFixtureStepIndex else koreanFixtureStepIndex
      val steps = banks[bankIndex]
      val index = stepIndex.coerceIn(0, steps.lastIndex)
      val text = steps[index]
      val isLast = index == steps.lastIndex
      if (isLast) {
        client.commitText(text)
        if (isJapanese) {
          japaneseFixtureStepIndex = 0
          japaneseFixtureBankIndex = (bankIndex + 1) % banks.size
        } else {
          koreanFixtureStepIndex = 0
          koreanFixtureBankIndex = (bankIndex + 1) % banks.size
        }
        activeFixtureLanguage = null
      } else {
        client.setComposingText(text)
        if (isJapanese) {
          japaneseFixtureStepIndex = index + 1
        } else {
          koreanFixtureStepIndex = index + 1
        }
        activeFixtureLanguage = isJapanese
      }
    }
  }

  private fun resetFixtureProgress(isJapanese: Boolean? = null) {
    when (isJapanese) {
      true -> japaneseFixtureStepIndex = 0
      false -> koreanFixtureStepIndex = 0
      null -> {
        japaneseFixtureStepIndex = 0
        koreanFixtureStepIndex = 0
      }
    }

    if (isJapanese == null || activeFixtureLanguage == isJapanese) {
      activeFixtureLanguage = null
    }
  }

  private fun activeClient(): TextInputClient? {
    val owner = activeOwner ?: focusedOwners.lastOrNull { clients.containsKey(it) } ?: return null
    activeOwner = owner
    return clients[owner]?.also { updateRecentClient(it) }
  }

  private fun dispatchTypedChar(char: Char) {
    dispatchText(char.toString())
  }

  private fun dispatchText(text: String) {
    resetFixtureProgress()
    withRefocusedClient { client ->
      scope.launch {
        text.forEachIndexed { index, char ->
          if (index > 0) {
            delay(12)
          }

          if (client.insertText(char.toString())) {
            return@forEachIndexed
          }

          dispatchKeyStroke(
            keyCode = KeyEvent.getExtendedKeyCodeForChar(char.code),
            typedChar = char,
          )
        }
      }
    }
  }

  private fun dispatchSpecialKey(keyCode: Int, directKey: TextInputKey? = null) {
    resetFixtureProgress()
    withRefocusedClient { client ->
      if (directKey != null && client.pressKey(directKey)) {
        return@withRefocusedClient
      }

      dispatchKeyStroke(keyCode = keyCode, typedChar = null)
    }
  }

  private fun dispatchKeyStroke(keyCode: Int, typedChar: Char?) {
    SwingUtilities.invokeLater {
      val focusTarget = currentKeyEventTarget() ?: return@invokeLater
      val at = System.currentTimeMillis()
      focusTarget.dispatchEvent(
        KeyEvent(focusTarget, KeyEvent.KEY_PRESSED, at, 0, keyCode, KeyEvent.CHAR_UNDEFINED)
      )
      if (typedChar != null) {
        focusTarget.dispatchEvent(
          KeyEvent(focusTarget, KeyEvent.KEY_TYPED, at, 0, KeyEvent.VK_UNDEFINED, typedChar)
        )
      }
      focusTarget.dispatchEvent(
        KeyEvent(focusTarget, KeyEvent.KEY_RELEASED, at, 0, keyCode, KeyEvent.CHAR_UNDEFINED)
      )
    }
  }

  private fun currentKeyEventTarget(): Component? =
    KeyboardFocusManager.getCurrentKeyboardFocusManager().focusOwner

  private fun rememberedClient(): TextInputClient? = activeClient() ?: recentClient

  private fun updateRecentClient(client: TextInputClient) {
    clearRecentClientJob?.cancel()
    clearRecentClientJob = null
    recentClient = client
  }

  private fun keepRecentClientBriefly(client: TextInputClient) {
    updateRecentClient(client)
    clearRecentClientJob = scope.launch {
      delay(DesktopDebugKeyboardHideDelayMillis + DesktopDebugKeyboardAnimationDurationMillis)
      if (focusedOwners.isEmpty() && !isPointerInteracting) {
        clearRecentClient()
      }
    }
  }

  private fun clearRecentClient() {
    clearRecentClientJob?.cancel()
    clearRecentClientJob = null
    recentClient = null
  }

  private fun withRefocusedClient(action: (TextInputClient) -> Unit) {
    val client = rememberedClient() ?: return
    hideJob?.cancel()
    if (!hardwareKeyboardConnected) {
      isVisible = true
    }
    client.requestFocus()
    scope.launch {
      delay(24)
      action(client)
    }
  }

  private fun scheduleHideIfNeeded() {
    hideJob?.cancel()
    hideJob = scope.launch {
      delay(DesktopDebugKeyboardHideDelayMillis)
      if (!isPointerInteracting && focusedOwners.isEmpty()) {
        clearRecentClient()
        resetFixtureProgress()
        isVisible = false
      }
    }
  }

  @Composable
  private fun animatedHeight(): Dp {
    val animatedHeight by
      animateDpAsState(
        targetValue = if (isVisible) height else 0.dp,
        animationSpec = tween(durationMillis = DesktopDebugKeyboardAnimationDurationMillis),
      )
    return animatedHeight
  }

  @Composable
  private fun KeyboardRow(keys: List<ActionKeySpec>) {
    Row(
      modifier = Modifier.fillMaxWidth(),
      horizontalArrangement = Arrangement.spacedBy(8.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      keys.forEach { spec ->
        KeyboardKey(spec = spec, modifier = Modifier.weight(spec.weight).height(48.dp))
      }
    }
  }

  @Composable
  private fun KeyboardKey(spec: ActionKeySpec, modifier: Modifier = Modifier) {
    val shape = AppShapes.rounded(AppShapes.sm)
    val interactionSource = remember { MutableInteractionSource() }
    val scope = rememberCoroutineScope()
    var isHovered by remember { mutableStateOf(false) }

    LaunchedEffect(interactionSource) {
      interactionSource.interactions.collect { interaction ->
        when (interaction) {
          is HoverInteraction.Enter -> isHovered = true
          is HoverInteraction.Exit -> isHovered = false
        }
      }
    }

    CompositionLocalProvider(LocalInteractionSource provides interactionSource) {
      Box(
        modifier =
          modifier
            .clip(shape)
            .background(spec.color)
            .border(1.dp, AppTheme.colors.borderHairline, shape)
            .hoverable(interactionSource)
            .pointerInput(spec.label, spec.repeatable) {
              awaitEachGesture {
                val down = awaitFirstDown(requireUnconsumed = false)
                val pressInteraction = PressInteraction.Press(down.position)
                scope.launch { interactionSource.emit(pressInteraction) }
                spec.onClick()

                val repeatJob =
                  if (spec.repeatable) {
                    scope.launch {
                      delay(DesktopDebugKeyboardRepeatDelayMillis)
                      while (true) {
                        spec.onClick()
                        delay(DesktopDebugKeyboardRepeatIntervalMillis)
                      }
                    }
                  } else {
                    null
                  }

                val up = waitForUpOrCancellation()
                repeatJob?.cancel()
                scope.launch {
                  interactionSource.emit(
                    if (up != null) {
                      PressInteraction.Release(pressInteraction)
                    } else {
                      PressInteraction.Cancel(pressInteraction)
                    }
                  )
                }
              }
            }
            .pressScale(0.96f),
        contentAlignment = Alignment.Center,
      ) {
        if (isHovered) {
          Box(modifier = Modifier.matchParentSize().background(DesktopDebugKeyboardHover, shape))
        }

        BasicText(
          text = spec.label,
          modifier = Modifier.fillMaxWidth().padding(horizontal = 6.dp),
          style =
            TextStyle(
              fontSize = 12.sp,
              fontWeight = FontWeight.SemiBold,
              color = AppTheme.colors.textMuted,
              textAlign = TextAlign.Center,
            ),
        )
      }
    }
  }
}

private data class ActionKeySpec(
  val label: String,
  val color: Color,
  val weight: Float = 1f,
  val repeatable: Boolean = false,
  val onClick: () -> Unit,
)
