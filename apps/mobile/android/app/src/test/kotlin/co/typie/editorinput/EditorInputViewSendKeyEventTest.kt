package co.typie.editorinput

import android.content.Context
import android.text.Selection
import android.text.Spanned
import android.view.InputDevice
import android.view.KeyCharacterMap
import android.view.KeyEvent
import android.view.inputmethod.CorrectionInfo
import android.view.inputmethod.EditorInfo
import androidx.test.core.app.ApplicationProvider
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.plugin.common.StandardMethodCodec
import java.nio.ByteBuffer
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], manifest = Config.NONE)
class EditorInputViewSendKeyEventTest {

  @Test
  fun `가상 비소프트 shift slash 는 물음표를 insertText 로 브리지한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    val event = createSlashEvent(
      flags = 0,
      metaState = KeyEvent.META_SHIFT_ON,
      deviceId = KeyCharacterMap.VIRTUAL_KEYBOARD
    )
    val handled = connection.sendKeyEvent(event)

    assertEquals(true, handled)
    val insertTextCall = messenger.calls.first { it.method == "insertText" }
    @Suppress("UNCHECKED_CAST")
    val args = insertTextCall.arguments as Map<String, Any?>
    assertEquals("?", args["text"])
  }

  @Test
  fun `하드웨어 onKeyDown 출력 가능 문자는 insertText 로 브리지한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)

    val event =
      createHardwareKeyEvent(
        keyCode = KeyEvent.KEYCODE_4,
        metaState = 0
      )

    val handled = view.onKeyDown(KeyEvent.KEYCODE_4, event)

    assertTrue(handled)
    val insertTextCall = messenger.calls.first { it.method == "insertText" }
    @Suppress("UNCHECKED_CAST")
    val args = insertTextCall.arguments as Map<String, Any?>
    assertEquals("4", args["text"])
  }

  @Test
  fun `하드웨어 onKeyDown shift slash 는 물음표를 브리지한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)

    val event =
      createHardwareKeyEvent(
        keyCode = KeyEvent.KEYCODE_SLASH,
        metaState = KeyEvent.META_SHIFT_ON
      )

    val handled = view.onKeyDown(KeyEvent.KEYCODE_SLASH, event)

    assertTrue(handled)
    val insertTextCall = messenger.calls.first { it.method == "insertText" }
    @Suppress("UNCHECKED_CAST")
    val args = insertTextCall.arguments as Map<String, Any?>
    assertEquals("?", args["text"])
  }

  @Test
  fun `하드웨어 onKeyDown 기호 매트릭스는 insertText 로 브리지한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)

    val cases =
      listOf(
        Triple(KeyEvent.KEYCODE_GRAVE, 0, "`"),
        Triple(KeyEvent.KEYCODE_GRAVE, KeyEvent.META_SHIFT_ON, "~"),
        Triple(KeyEvent.KEYCODE_1, 0, "1"),
        Triple(KeyEvent.KEYCODE_2, 0, "2"),
        Triple(KeyEvent.KEYCODE_1, KeyEvent.META_SHIFT_ON, "!"),
        Triple(KeyEvent.KEYCODE_2, KeyEvent.META_SHIFT_ON, "@"),
        Triple(KeyEvent.KEYCODE_MINUS, 0, "-"),
        Triple(KeyEvent.KEYCODE_BACKSLASH, 0, "\\"),
        Triple(KeyEvent.KEYCODE_BACKSLASH, KeyEvent.META_SHIFT_ON, "|"),
        Triple(KeyEvent.KEYCODE_SLASH, 0, "/"),
        Triple(KeyEvent.KEYCODE_SLASH, KeyEvent.META_SHIFT_ON, "?"),
        Triple(KeyEvent.KEYCODE_PERIOD, 0, "."),
        Triple(KeyEvent.KEYCODE_COMMA, 0, ","),
        Triple(KeyEvent.KEYCODE_COMMA, KeyEvent.META_SHIFT_ON, "<"),
        Triple(KeyEvent.KEYCODE_PERIOD, KeyEvent.META_SHIFT_ON, ">")
      )

    val expected = mutableListOf<String>()
    cases.forEach { (keyCode, metaState, text) ->
      val event = createHardwareKeyEvent(keyCode = keyCode, metaState = metaState)
      val handled = view.onKeyDown(keyCode, event)
      assertTrue("expected handled for $keyCode/$metaState", handled)
      expected += text
    }

    val insertedTexts =
      messenger.calls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(expected, insertedTexts)
  }

  @Test
  fun `소프트 키보드 문장부호 이벤트는 printable 브리지 정책에서 제외된다`() {
    val shouldBridge =
      EditorInputNativeView.shouldBridgePrintableKey(
        keyCode = KeyEvent.KEYCODE_SLASH,
        unicode = '?'.code,
        isCtrlPressed = false,
        isAltPressed = false,
        isMetaPressed = false,
        isSoftKeyboardEvent = true
      )

    assertFalse(shouldBridge)
  }

  @Test
  fun `소프트 키보드 숫자 이벤트는 printable 브리지 정책에 포함된다`() {
    val shouldBridge =
      EditorInputNativeView.shouldBridgePrintableKey(
        keyCode = KeyEvent.KEYCODE_4,
        unicode = '4'.code,
        isCtrlPressed = false,
        isAltPressed = false,
        isMetaPressed = false,
        isSoftKeyboardEvent = true
      )

    assertTrue(shouldBridge)
  }

  @Test
  fun `Gboard 한글 조합 커밋은 중복 삽입 없이 다음 조합을 유지한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    val firstWordComposingFrames =
      listOf(
        "ㅎ",
        "하",
        "한",
        "한ㄱ",
        "한ㄱ",
        "한그",
        "한글"
      )
    firstWordComposingFrames.forEach { frame ->
      connection.setComposingText(frame, 1)
    }

    connection.commitText("한글", 1)
    connection.commitText(" ", 1)

    val secondWordComposingFrames =
      listOf(
        "ㅇ",
        "이",
        "입",
        "입ㄹ",
        "입ㄹ",
        "입러",
        "입러",
        "입려",
        "입려",
        "입력"
      )
    secondWordComposingFrames.forEach { frame ->
      connection.setComposingText(frame, 1)
    }

    val appCalls = messenger.calls
    val markTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(firstWordComposingFrames + secondWordComposingFrames, markTexts)
    assertEquals(listOf(" "), insertedTexts)
    assertFalse(insertedTexts.contains("한글"))
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })

    val unmarkIndex = appCalls.indexOfFirst { it.method == "unmarkText" }
    val spaceInsertIndex =
      appCalls.indexOfFirst { call ->
        if (call.method != "insertText") return@indexOfFirst false
        @Suppress("UNCHECKED_CAST")
        val args = call.arguments as Map<String, Any?>
        args["text"] == " "
      }
    assertTrue(unmarkIndex >= 0)
    assertTrue(spaceInsertIndex > unmarkIndex)
  }

  @Test
  fun `Gboard 단자음 공백 후 다음 자음 입력 시 마지막 프레임 조합을 유지한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.setComposingText("ㅎ", 1)
    connection.commitText("ㅎ", 1)
    connection.commitText(" ", 1)
    connection.setComposingText("ㅎ", 1)

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(listOf("setMarkedText", "unmarkText", "insertText", "setMarkedText"), callMethods)

    @Suppress("UNCHECKED_CAST")
    val firstMarkText = (appCalls[0].arguments as Map<String, Any?>)["text"] as String
    @Suppress("UNCHECKED_CAST")
    val insertedSpace = (appCalls[2].arguments as Map<String, Any?>)["text"] as String
    @Suppress("UNCHECKED_CAST")
    val secondMarkText = (appCalls[3].arguments as Map<String, Any?>)["text"] as String

    assertEquals("ㅎ", firstMarkText)
    assertEquals(" ", insertedSpace)
    assertEquals("ㅎ", secondMarkText)
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals(0, appCalls.count { call ->
      if (call.method != "insertText") return@count false
      @Suppress("UNCHECKED_CAST")
      val args = call.arguments as Map<String, Any?>
      args["text"] == "ㅎ"
    })
  }

  @Test
  fun `Gboard 단자음 공백 후 마지막 조합 자음 삭제 시 mark 만 취소한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.setComposingText("ㅎ", 1)
    connection.commitText("ㅎ", 1)
    connection.commitText(" ", 1)
    connection.setComposingText("ㅎ", 1)
    connection.setComposingText("", 1)

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(listOf("setMarkedText", "unmarkText", "insertText", "setMarkedText", "cancelMarkedText"), callMethods)

    @Suppress("UNCHECKED_CAST")
    val firstMarkText = (appCalls[0].arguments as Map<String, Any?>)["text"] as String
    @Suppress("UNCHECKED_CAST")
    val insertedSpace = (appCalls[2].arguments as Map<String, Any?>)["text"] as String
    @Suppress("UNCHECKED_CAST")
    val secondMarkText = (appCalls[3].arguments as Map<String, Any?>)["text"] as String

    assertEquals("ㅎ", firstMarkText)
    assertEquals(" ", insertedSpace)
    assertEquals("ㅎ", secondMarkText)
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(1, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals(0, appCalls.count { call ->
      if (call.method != "insertText") return@count false
      @Suppress("UNCHECKED_CAST")
      val args = call.arguments as Map<String, Any?>
      args["text"] == "ㅎ"
    })
    assertEquals("ㅎ ", view.text?.toString())
  }

  @Test
  fun `Gboard 점 세 개와 공백 뒤 한글 조합이 동작한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.commitText(".", 1)
    connection.commitText(".", 1)
    connection.commitText(".", 1)
    connection.commitText(" ", 1)
    connection.setComposingText("ㅎ", 1)
    connection.setComposingText("하", 1)

    val appCalls = messenger.calls
    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf(".", ".", ".", " "), insertedTexts)
    assertEquals(listOf("ㅎ", "하"), markedTexts)
    assertEquals(0, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("... 하", view.text?.toString())
  }

  @Test
  fun `Gboard 더블 스페이스 마침표가 이전 공백을 치환하고 조합을 이어간다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.setComposingText("ㅎ", 1)
    connection.setComposingText("하", 1)
    connection.setComposingText("한", 1)
    connection.commitText("한", 1)
    connection.commitText(" ", 1)

    connection.beginBatchEdit()
    connection.finishComposingText()
    connection.setSelection(8, 8)
    connection.deleteSurroundingText(1, 0)
    connection.commitText(". ", 1)
    connection.endBatchEdit()

    connection.setComposingText("ㄱ", 1)
    connection.setComposingText("그", 1)
    connection.setComposingText("글", 1)

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(
      listOf(
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "unmarkText",
        "insertText",
        "deleteBackward",
        "insertText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText"
      ),
      callMethods
    )

    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf(" ", ". "), insertedTexts)
    assertEquals(listOf("ㅎ", "하", "한", "ㄱ", "그", "글"), markedTexts)
    assertEquals(1, appCalls.count { it.method == "deleteBackward" })
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("한. 글", view.text?.toString())
  }

  @Test
  fun `딩굴 더블 스페이스 마침표가 이전 공백을 치환하고 조합을 이어간다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.beginBatchEdit()
    connection.setComposingText("ㅎ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("하", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("한", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.finishComposingText()
    connection.commitText(" ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.deleteSurroundingText(1, 0)
    connection.commitText(". ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("ㄱ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("그", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("글", 1)
    connection.endBatchEdit()

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(
      listOf(
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "unmarkText",
        "insertText",
        "deleteBackward",
        "insertText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText"
      ),
      callMethods
    )

    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf(" ", ". "), insertedTexts)
    assertEquals(listOf("ㅎ", "하", "한", "ㄱ", "그", "글"), markedTexts)
    assertEquals(1, appCalls.count { it.method == "deleteBackward" })
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("한. 글", view.text?.toString())
  }

  @Test
  fun `Gboard 일본어 로마자 변환은 조합 프레임 뒤 변환 텍스트를 커밋한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.beginBatchEdit()
    connection.setComposingText("ｎ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("に", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にｈ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にほ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にほｎ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にほんｇ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にほんご", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.commitText("日本語", 1)
    connection.endBatchEdit()

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(
      listOf(
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "cancelMarkedText",
        "insertText"
      ),
      callMethods
    )

    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf("ｎ", "に", "にｈ", "にほ", "にほｎ", "にほんｇ", "にほんご"), markedTexts)
    assertEquals(listOf("日本語"), insertedTexts)
    assertEquals(0, appCalls.count { it.method == "unmarkText" })
    assertEquals(1, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("日本語", view.text?.toString())
  }

  @Test
  fun `Gboard 일본어 로마자 조합은 축소되다가 모두 삭제되면 취소된다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.beginBatchEdit()
    connection.setComposingText("ｎ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("に", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にｈ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にほ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にほｎ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にほん", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("にほ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("に", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("", 1)
    connection.endBatchEdit()

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(
      listOf(
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "cancelMarkedText"
      ),
      callMethods
    )

    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf("ｎ", "に", "にｈ", "にほ", "にほｎ", "にほん", "にほ", "に"), markedTexts)
    assertEquals(0, appCalls.count { it.method == "insertText" })
    assertEquals(0, appCalls.count { it.method == "unmarkText" })
    assertEquals(1, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("", view.text?.toString())
  }

  @Test
  fun `Gboard 자동 교정은 되돌린 뒤 모두 지울 수 있다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.setComposingText("ㅇ", 1)
    connection.setComposingText("아", 1)
    connection.setComposingText("안", 1)
    connection.setComposingText("안ㄴ", 1)
    connection.setComposingText("안ㄴ", 1)
    connection.setComposingText("안너", 1)
    connection.setComposingText("안너", 1)
    connection.setComposingText("안넝", 1)
    connection.setComposingText("안넝", 1)
    connection.commitText("안녕", 1)
    connection.commitText(" ", 1)

    connection.setComposingRegion(0, 3)
    connection.setComposingText("안넝", 1)
    connection.setComposingText("안", 1)
    connection.setComposingText("", 1)

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(
      listOf(
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "cancelMarkedText",
        "insertText",
        "insertText",
        "deleteBackward",
        "deleteBackward",
        "deleteBackward",
        "setMarkedText",
        "setMarkedText",
        "cancelMarkedText"
      ),
      callMethods
    )

    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf("ㅇ", "아", "안", "안ㄴ", "안ㄴ", "안너", "안너", "안넝", "안넝", "안넝", "안"), markedTexts)
    assertEquals(listOf("안녕", " "), insertedTexts)
    assertEquals(3, appCalls.count { it.method == "deleteBackward" })
    assertEquals(0, appCalls.count { it.method == "unmarkText" })
    assertEquals(2, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("", view.text?.toString())
  }

  @Test
  fun `LG 키보드 commitCorrection 자동 교정은 되돌린 뒤 모두 지울 수 있다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.beginBatchEdit()
    connection.setComposingText("ㅇ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("아", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안ㄴ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안너", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안넝", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.commitText("안녕", 1)
    connection.commitCorrection(CorrectionInfo(0, "", "안녕"))
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.commitText(" ", 1)
    connection.endBatchEdit()

    val delDown = createSoftDeleteEvent(KeyEvent.ACTION_DOWN)
    val delUp = createSoftDeleteEvent(KeyEvent.ACTION_UP)
    connection.sendKeyEvent(delDown)
    connection.sendKeyEvent(delUp)

    connection.setComposingRegion(0, 2)

    connection.beginBatchEdit()
    connection.setComposingText("안넝", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("", 1)
    connection.finishComposingText()
    connection.endBatchEdit()

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(
      listOf(
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "cancelMarkedText",
        "insertText",
        "insertText",
        "deleteBackward",
        "deleteBackward",
        "deleteBackward",
        "setMarkedText",
        "setMarkedText",
        "cancelMarkedText"
      ),
      callMethods
    )

    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf("ㅇ", "아", "안", "안ㄴ", "안너", "안넝", "안넝", "안"), markedTexts)
    assertEquals(listOf("안녕", " "), insertedTexts)
    assertEquals(3, appCalls.count { it.method == "deleteBackward" })
    assertEquals(0, appCalls.count { it.method == "unmarkText" })
    assertEquals(2, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("", view.text?.toString())
  }

  @Test
  fun `LG 키보드는 조합 후 공백과 백스페이스로 같은 단어 조합을 재개한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.beginBatchEdit()
    connection.setComposingText("ㅇ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("아", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안ㄴ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안너", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안녀", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("안녕", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.finishComposingText()
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.commitText(" ", 1)
    connection.endBatchEdit()

    val delDown = createSoftDeleteEvent(KeyEvent.ACTION_DOWN)
    val delUp = createSoftDeleteEvent(KeyEvent.ACTION_UP)
    connection.sendKeyEvent(delDown)
    connection.sendKeyEvent(delUp)

    connection.setComposingRegion(0, 2)

    connection.beginBatchEdit()
    connection.setComposingText("안녕", 1)
    connection.endBatchEdit()

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(
      listOf(
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "setMarkedText",
        "unmarkText",
        "insertText",
        "deleteBackward",
        "deleteBackward",
        "deleteBackward",
        "setMarkedText"
      ),
      callMethods
    )

    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf("ㅇ", "아", "안", "안ㄴ", "안너", "안녀", "안녕", "안녕"), markedTexts)
    assertEquals(listOf(" "), insertedTexts)
    assertEquals(3, appCalls.count { it.method == "deleteBackward" })
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("안녕", view.text?.toString())
  }

  @Test
  fun `스마트보드 플로팅 커서 좌우 이동은 한글 조합 커밋 후에만 navigate 를 보낸다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    val composingFrames =
      listOf(
        "ㅇ",
        "아",
        "안",
        "안ㄴ",
        "안너",
        "안녀",
        "안녕",
        "안녕ㅎ",
        "안녕하",
        "안녕핫",
        "안녕하세",
        "안녕하셍",
        "안녕하세오",
        "안녕하세요"
      )
    composingFrames.forEach { frame ->
      connection.beginBatchEdit()
      connection.setComposingText(frame, 1)
      connection.endBatchEdit()
    }

    connection.beginBatchEdit()
    connection.finishComposingText()
    connection.endBatchEdit()

    repeat(5) {
      val leftDown = createSoftDpadEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_LEFT)
      val leftUp = createSoftDpadEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_DPAD_LEFT)
      connection.sendKeyEvent(leftDown)
      connection.sendKeyEvent(leftUp)

      connection.beginBatchEdit()
      connection.finishComposingText()
      connection.endBatchEdit()
    }

    repeat(10) {
      val rightDown = createSoftDpadEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_RIGHT)
      val rightUp = createSoftDpadEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_DPAD_RIGHT)
      connection.sendKeyEvent(rightDown)
      connection.sendKeyEvent(rightUp)

      connection.beginBatchEdit()
      connection.finishComposingText()
      connection.endBatchEdit()
    }

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(
      List(composingFrames.size) { "setMarkedText" } + listOf("unmarkText") + List(15) { "navigate" },
      callMethods
    )

    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val navigateDirections =
      appCalls
        .filter { it.method == "navigate" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["direction"] as String
        }
    val navigateExtendFlags =
      appCalls
        .filter { it.method == "navigate" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["extend"] as Boolean
        }

    assertEquals(composingFrames, markedTexts)
    assertEquals(List(5) { "left" } + List(10) { "right" }, navigateDirections)
    assertEquals(List(15) { false }, navigateExtendFlags)
    assertEquals(0, appCalls.count { it.method == "insertText" })
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("안녕하세요", view.text?.toString())
  }

  @Test
  fun `소프트 DPAD ACTION_MULTIPLE 은 추가 navigate 를 브리지하지 않는다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.beginBatchEdit()
    connection.setComposingText("안녕하세요", 1)
    connection.endBatchEdit()

    val leftDown = createSoftDpadEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_LEFT, downTime = 10L, eventTime = 10L)
    val leftMultiple = createSoftDpadEvent(2, KeyEvent.KEYCODE_DPAD_LEFT, repeatCount = 3, downTime = 10L, eventTime = 20L)
    val leftUp = createSoftDpadEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_DPAD_LEFT, downTime = 10L, eventTime = 30L)
    connection.sendKeyEvent(leftDown)
    connection.sendKeyEvent(leftMultiple)
    connection.sendKeyEvent(leftUp)

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(listOf("setMarkedText", "unmarkText", "navigate"), callMethods)

    val navigateDirections =
      appCalls
        .filter { it.method == "navigate" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["direction"] as String
        }

    assertEquals(listOf("left"), navigateDirections)
    assertEquals(0, appCalls.count { it.method == "insertText" })
    assertEquals(0, appCalls.count { it.method == "deleteBackward" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("안녕하세요", view.text?.toString())
  }

  @Test
  fun `LG 플로팅 커서 추정 문맥 조회 직후 좌우 이동은 조합을 확정하고 navigate 를 보낸다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.setComposingText("ㅇ", 1)
    connection.getTextBeforeCursor(4096, 0)
    connection.getTextAfterCursor(4096, 0)

    val leftDown = createSoftDpadEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_LEFT)
    val leftUp = createSoftDpadEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_DPAD_LEFT)
    connection.sendKeyEvent(leftDown)
    connection.sendKeyEvent(leftUp)

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(listOf("setMarkedText", "unmarkText", "navigate"), callMethods)
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals(0, appCalls.count { it.method == "insertText" })
    assertEquals(0, appCalls.count { it.method == "deleteBackward" })
    assertEquals("ㅇ", view.text?.toString())
  }

  @Test
  fun `삼성 플로팅 커서 상좌우 이동은 텍스트 변경 없이 navigate 한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    val composingFrames =
      listOf(
        "ㅇ",
        "아",
        "안",
        "안ㄴ",
        "안너",
        "안녀",
        "안녕",
        "안녕ㅎ",
        "안녕하",
        "안녕핫",
        "안녕하세",
        "안녕하셍",
        "안녕하세오",
        "안녕하세요"
      )
    composingFrames.forEach { frame ->
      connection.setComposingText(frame, 1)
    }

    val upDown = createSoftDpadEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_UP)
    val upUp = createSoftDpadEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_DPAD_UP)
    connection.sendKeyEvent(upDown)
    connection.sendKeyEvent(upUp)

    connection.finishComposingText()
    connection.finishComposingText()

    repeat(6) {
      val leftDown = createSoftDpadEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_LEFT)
      val leftUp = createSoftDpadEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_DPAD_LEFT)
      connection.sendKeyEvent(leftDown)
      connection.sendKeyEvent(leftUp)
    }

    repeat(8) {
      val rightDown = createSoftDpadEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DPAD_RIGHT)
      val rightUp = createSoftDpadEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_DPAD_RIGHT)
      connection.sendKeyEvent(rightDown)
      connection.sendKeyEvent(rightUp)
    }

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(
      List(composingFrames.size) { "setMarkedText" } + listOf("unmarkText") + List(15) { "navigate" },
      callMethods
    )

    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val navigateDirections =
      appCalls
        .filter { it.method == "navigate" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["direction"] as String
        }
    val navigateExtendFlags =
      appCalls
        .filter { it.method == "navigate" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["extend"] as Boolean
        }

    assertEquals(composingFrames, markedTexts)
    assertEquals(listOf("up") + List(6) { "left" } + List(8) { "right" }, navigateDirections)
    assertEquals(List(15) { false }, navigateExtendFlags)
    assertEquals(0, appCalls.count { it.method == "insertText" })
    assertEquals(0, appCalls.count { it.method == "deleteBackward" })
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("안녕하세요", view.text?.toString())
  }

  @Test
  fun `LG 키보드는 빈 문서의 stale 조합 영역에서 이전 텍스트를 재삽입하지 않는다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.setComposingText("글", 1)

    connection.beginBatchEdit()
    connection.setComposingText("", 1)
    connection.finishComposingText()
    connection.endBatchEdit()

    val firstDelDown = createSoftDeleteEvent(KeyEvent.ACTION_DOWN)
    val firstDelUp = createSoftDeleteEvent(KeyEvent.ACTION_UP)
    connection.sendKeyEvent(firstDelDown)
    connection.sendKeyEvent(firstDelUp)

    connection.setComposingRegion(16, 18)
    seedCollapsedComposingSpan(view)
    connection.beginBatchEdit()
    connection.setComposingText("한.", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("한", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("", 1)
    connection.finishComposingText()
    connection.endBatchEdit()

    val secondDelDown = createSoftDeleteEvent(KeyEvent.ACTION_DOWN)
    val secondDelUp = createSoftDeleteEvent(KeyEvent.ACTION_UP)
    connection.sendKeyEvent(secondDelDown)
    connection.sendKeyEvent(secondDelUp)

    connection.setComposingRegion(14, 15)
    seedCollapsedComposingSpan(view)
    connection.beginBatchEdit()
    connection.setComposingText("글", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("", 1)
    connection.finishComposingText()
    connection.endBatchEdit()

    connection.finishComposingText()
    connection.finishComposingText()

    val appCalls = messenger.calls
    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf("글", "한"), markedTexts)
    assertEquals(emptyList<String>(), insertedTexts)
    assertEquals(2, appCalls.count { it.method == "deleteBackward" })
    assertEquals(0, appCalls.count { it.method == "unmarkText" })
    assertEquals(2, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("", view.text?.toString())
  }

  @Test
  fun `편집 가능 텍스트보다 긴 stale 조합 영역은 delete 브리지를 소비하지 않는다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    view.setText("x")
    view.setSelection(1)

    connection.setComposingRegion(16, 18)

    val editable = view.text!!
    editable.setSpan(Any(), 0, 1, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE or Spanned.SPAN_COMPOSING)
    Selection.setSelection(editable, 1)

    connection.beginBatchEdit()
    connection.setComposingText("한.", 1)
    connection.endBatchEdit()

    val appCalls = messenger.calls
    assertEquals(0, appCalls.count { it.method == "deleteBackward" })
    assertEquals(0, appCalls.count { it.method == "setMarkedText" })
    assertEquals("x", view.text?.toString())
  }

  @Test
  fun `LG 문서 시작점 백스페이스는 이전 조합 텍스트를 재삽입하지 않는다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.beginBatchEdit()
    connection.setComposingText("한글", 1)
    connection.endBatchEdit()
    view.resetInputContext()

    view.setSelection(0)
    val delDown = createSoftDeleteEvent(KeyEvent.ACTION_DOWN)
    val delUp = createSoftDeleteEvent(KeyEvent.ACTION_UP)
    connection.sendKeyEvent(delDown)
    connection.sendKeyEvent(delUp)

    val sizeAfterDelete = messenger.calls.size

    connection.setComposingRegion(1, 2)
    connection.beginBatchEdit()
    connection.setComposingText("한", 1)
    connection.endBatchEdit()
    connection.finishComposingText()

    val appCalls = messenger.calls
    val replayCalls = appCalls.drop(sizeAfterDelete)

    assertEquals(0, replayCalls.count { it.method == "setMarkedText" })
    assertEquals(0, replayCalls.count { it.method == "deleteBackward" })
    assertEquals(0, replayCalls.count { it.method == "insertText" })
    assertEquals(0, replayCalls.count { it.method == "cancelMarkedText" })
    assertEquals("한글", view.text?.toString())
  }

  @Test
  fun `삼성 커서 이동 후 조합은 앞쪽 텍스트를 지우지 않고 조합을 이어간다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    val helloFrames =
      listOf(
        "ㅇ",
        "아",
        "안",
        "안ㄴ",
        "안너",
        "안녀",
        "안녕",
        "안녕ㅎ",
        "안녕하",
        "안녕핫",
        "안녕하세",
        "안녕하셍",
        "안녕하세오",
        "안녕하세요"
      )
    helloFrames.forEach { frame ->
      connection.setComposingText(frame, 1)
    }
    view.resetInputContext()

    connection.finishComposingText()
    connection.setSelection(2, 2)
    connection.setComposingRegion(0, 5)

    val middleFrames =
      listOf(
        "안녕하세요ㅂ",
        "안녕하세요바",
        "안녕하세요반",
        "안녕하세요반ㄱ",
        "안녕하세요반가",
        "안녕하세요반갑",
        "안녕하세요반값",
        "안녕하세요반갑스",
        "안녕하세요반갑습",
        "안녕하세요반갑습ㄴ",
        "안녕하세요반갑습니",
        "안녕하세요반갑습닏",
        "안녕하세요반갑습니다"
      )
    middleFrames.forEach { frame ->
      connection.setComposingText(frame, 1)
    }
    view.resetInputContext()

    connection.finishComposingText()
    val end = view.text?.length ?: 0
    connection.setSelection(end, end)
    connection.setComposingRegion(0, 10)

    val tailFrames =
      listOf(
        "안녕하세요반갑습니다ㅎ",
        "안녕하세요반갑습니다하",
        "안녕하세요반갑습니다항",
        "안녕하세요반갑습니다하이"
      )
    tailFrames.forEach { frame ->
      connection.setComposingText(frame, 1)
    }
    view.resetInputContext()

    val appCalls = messenger.calls
    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(0, appCalls.count { it.method == "deleteBackward" })
    assertEquals(0, appCalls.count { it.method == "insertText" })
    assertEquals(3, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals(
      helloFrames +
        listOf(
          "ㅂ",
          "바",
          "반",
          "반ㄱ",
          "반가",
          "반갑",
          "반값",
          "반갑스",
          "반갑습",
          "반갑습ㄴ",
          "반갑습니",
          "반갑습닏",
          "반갑습니다",
          "ㅎ",
          "하",
          "항",
          "하이"
        ),
      markedTexts
    )
  }

  @Test
  fun `디자인 키보드 천지인은 문장을 조합하고 엔터는 줄바꿈 액션을 수행한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.beginBatchEdit()
    connection.setComposingText("ㄴ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(0, 1)
    connection.setComposingText("니", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(0, 1)
    connection.setComposingText("나", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(0, 1)
    connection.setComposingText("난", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(0, 1)
    connection.setComposingText("나느", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(0, 2)
    connection.setComposingText("나는", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(0, 2)
    connection.setComposingText("나는", 1)
    connection.endBatchEdit()

    connection.finishComposingText()
    connection.commitText(" ", 1)

    connection.beginBatchEdit()
    connection.setComposingText("ㅅ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 4)
    connection.setComposingText("시", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 4)
    connection.setComposingText("사", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 4)
    connection.setComposingText("산", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 4)
    connection.setComposingText("살", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 4)
    connection.setComposingText("살ㅇ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 5)
    connection.setComposingText("살이", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 5)
    connection.setComposingText("살아", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 5)
    connection.setComposingText("살앙", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 5)
    connection.setComposingText("살아이", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 6)
    connection.setComposingText("살아잇", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 6)
    connection.setComposingText("살아잏", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 6)
    connection.setComposingText("살아있", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 6)
    connection.setComposingText("살아있ㄷ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 7)
    connection.setComposingText("살아있디", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingRegion(3, 7)
    connection.setComposingText("살아있다", 1)
    connection.endBatchEdit()

    connection.finishComposingText()

    val enterDown = createSoftEnterEvent(KeyEvent.ACTION_DOWN)
    val enterUp = createSoftEnterEvent(KeyEvent.ACTION_UP)
    connection.beginBatchEdit()
    connection.sendKeyEvent(enterDown)
    connection.sendKeyEvent(enterUp)
    connection.endBatchEdit()

    val appCalls = messenger.calls
    val markedTexts =
      appCalls
        .filter { it.method == "setMarkedText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }
    val performActions =
      appCalls
        .filter { it.method == "performAction" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["action"] as String
        }

    assertEquals(
      listOf(
        "ㄴ",
        "니",
        "나",
        "난",
        "나느",
        "나는",
        "나는",
        "ㅅ",
        "시",
        "사",
        "산",
        "살",
        "살ㅇ",
        "살이",
        "살아",
        "살앙",
        "살아이",
        "살아잇",
        "살아잏",
        "살아있",
        "살아있ㄷ",
        "살아있디",
        "살아있다"
      ),
      markedTexts
    )
    assertEquals(listOf(" "), insertedTexts)
    assertEquals(listOf("newline"), performActions)
    assertEquals(2, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals("나는 살아있다\n", view.text?.toString())
  }

  @Test
  fun `딩굴 배치 편집 조합 공백 조합은 마지막 자모 조합 상태를 유지한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    connection.beginBatchEdit()
    connection.setComposingText("ㅋ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.finishComposingText()
    connection.commitText(" ", 1)
    connection.endBatchEdit()

    connection.beginBatchEdit()
    connection.setComposingText("ㅋ", 1)
    connection.endBatchEdit()

    val appCalls = messenger.calls
    val callMethods = appCalls.map { it.method }
    assertEquals(listOf("setMarkedText", "unmarkText", "insertText", "setMarkedText"), callMethods)

    @Suppress("UNCHECKED_CAST")
    val firstMarkText = (appCalls[0].arguments as Map<String, Any?>)["text"] as String
    @Suppress("UNCHECKED_CAST")
    val insertedSpace = (appCalls[2].arguments as Map<String, Any?>)["text"] as String
    @Suppress("UNCHECKED_CAST")
    val secondMarkText = (appCalls[3].arguments as Map<String, Any?>)["text"] as String

    assertEquals("ㅋ", firstMarkText)
    assertEquals(" ", insertedSpace)
    assertEquals("ㅋ", secondMarkText)
    assertEquals(1, appCalls.count { it.method == "unmarkText" })
    assertEquals(0, appCalls.count { it.method == "cancelMarkedText" })
    assertEquals(0, appCalls.count { call ->
      if (call.method != "insertText") return@count false
      @Suppress("UNCHECKED_CAST")
      val args = call.arguments as Map<String, Any?>
      args["text"] == "ㅋ"
    })
  }

  @Test
  fun `딩굴 소프트 키보드 숫자 이벤트는 채널로 브리지되고 editable 을 갱신한다`() {
    val messenger = RecordingBinaryMessenger()
    val channel = MethodChannel(messenger, "co.typie.editor_input.test")
    val context = ApplicationProvider.getApplicationContext<Context>()
    val view = EditorInputNativeView(context, channel)
    val connection = view.onCreateInputConnection(EditorInfo())

    val down = createSoftDigitEvent(KeyEvent.ACTION_DOWN)
    val up = createSoftDigitEvent(KeyEvent.ACTION_UP)

    connection.beginBatchEdit()
    connection.sendKeyEvent(down)
    connection.sendKeyEvent(up)
    connection.endBatchEdit()
    view.onKeyDown(KeyEvent.KEYCODE_4, down)

    val appCalls = messenger.calls
    val insertedTexts =
      appCalls
        .filter { it.method == "insertText" }
        .map {
          @Suppress("UNCHECKED_CAST")
          val args = it.arguments as Map<String, Any?>
          args["text"] as String
        }

    assertEquals(listOf("4"), insertedTexts)
    assertEquals(0, appCalls.count { it.method == "setMarkedText" })
    assertEquals(0, appCalls.count { it.method == "unmarkText" })
    assertEquals("4", view.text?.toString())
  }

  private fun createSlashEvent(flags: Int, metaState: Int, deviceId: Int): KeyEvent {
    return KeyEvent(
      0L,
      0L,
      KeyEvent.ACTION_DOWN,
      KeyEvent.KEYCODE_SLASH,
      0,
      metaState,
      deviceId,
      0,
      flags,
      InputDevice.SOURCE_KEYBOARD
    )
  }

  private fun createHardwareKeyEvent(keyCode: Int, metaState: Int): KeyEvent {
    return KeyEvent(
      0L,
      0L,
      KeyEvent.ACTION_DOWN,
      keyCode,
      0,
      metaState,
      7,
      0,
      0,
      InputDevice.SOURCE_KEYBOARD
    )
  }

  private fun createSoftDigitEvent(action: Int): KeyEvent {
    return KeyEvent(
      0L,
      0L,
      action,
      KeyEvent.KEYCODE_4,
      0,
      0,
      KeyCharacterMap.VIRTUAL_KEYBOARD,
      0,
      KeyEvent.FLAG_SOFT_KEYBOARD,
      InputDevice.SOURCE_KEYBOARD
    )
  }

  private fun createSoftEnterEvent(action: Int): KeyEvent {
    return KeyEvent(
      0L,
      0L,
      action,
      KeyEvent.KEYCODE_ENTER,
      0,
      0,
      KeyCharacterMap.VIRTUAL_KEYBOARD,
      0,
      KeyEvent.FLAG_SOFT_KEYBOARD,
      InputDevice.SOURCE_KEYBOARD
    )
  }

  private fun createSoftDeleteEvent(action: Int): KeyEvent {
    return KeyEvent(
      0L,
      0L,
      action,
      KeyEvent.KEYCODE_DEL,
      0,
      0,
      KeyCharacterMap.VIRTUAL_KEYBOARD,
      0,
      KeyEvent.FLAG_SOFT_KEYBOARD,
      InputDevice.SOURCE_KEYBOARD
    )
  }

  private fun createSoftDpadEvent(
    action: Int,
    keyCode: Int,
    repeatCount: Int = 0,
    downTime: Long = 0L,
    eventTime: Long = 0L
  ): KeyEvent {
    return KeyEvent(
      downTime,
      eventTime,
      action,
      keyCode,
      repeatCount,
      0,
      KeyCharacterMap.VIRTUAL_KEYBOARD,
      0,
      KeyEvent.FLAG_SOFT_KEYBOARD,
      InputDevice.SOURCE_KEYBOARD
    )
  }

  private fun seedCollapsedComposingSpan(view: EditorInputNativeView) {
    val editable = view.text ?: return
    val cursor = Selection.getSelectionStart(editable).let { if (it < 0) editable.length else it }.coerceIn(0, editable.length)
    editable.setSpan(Any(), cursor, cursor, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE or Spanned.SPAN_COMPOSING)
    Selection.setSelection(editable, cursor)
  }

  private data class OutgoingMethodCall(val method: String, val arguments: Any?)

  private class RecordingBinaryMessenger : BinaryMessenger {
    val calls = mutableListOf<OutgoingMethodCall>()

    override fun send(channel: String, message: ByteBuffer?) {
      record(message)
    }

    override fun send(channel: String, message: ByteBuffer?, callback: BinaryMessenger.BinaryReply?) {
      record(message)
      callback?.reply(null)
    }

    override fun setMessageHandler(channel: String, handler: BinaryMessenger.BinaryMessageHandler?) {
    }

    private fun record(message: ByteBuffer?) {
      if (message == null) return
      val buffer = message.duplicate()
      buffer.flip()
      val call: MethodCall = StandardMethodCodec.INSTANCE.decodeMethodCall(buffer)
      calls += OutgoingMethodCall(method = call.method, arguments = call.arguments)
    }
  }
}
