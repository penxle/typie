package co.typie.screen.editor_settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import co.touchlab.kermit.Logger
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.EditorSettingsScreen_Query
import co.typie.graphql.EditorSettingsScreen_UpdatePreferences_Mutation
import co.typie.graphql.GraphQLViewModel
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.QueryState
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdatePreferencesInput
import co.typie.graphql.type.buildUser
import co.typie.overlay.Loader
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.service.EditorPreferencesService
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.SettingControlRow
import co.typie.ui.component.SettingSwitch
import co.typie.ui.component.Text
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CancellationException
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.jsonPrimitive
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel
import org.koin.core.annotation.KoinViewModel
import kotlin.math.roundToInt

@Composable
fun EditorSettingsScreen() {
  val model = koinViewModel<EditorSettingsViewModel>()
  val editorPreferences = koinInject<EditorPreferencesService>()
  val bottomSheetHost = LocalBottomSheetHost.current
  val scrollState = rememberScrollState()

  val typewriterEnabled = editorPreferences.typewriterEnabled
  val typewriterPosition = editorPreferences.typewriterPosition
  val lineHighlightEnabled = editorPreferences.lineHighlightEnabled
  val autoSurroundEnabled = editorPreferences.autoSurroundEnabled
  val characterCountFloatingEnabled = editorPreferences.characterCountFloatingEnabled
  val widgetAutoFadeEnabled = editorPreferences.widgetAutoFadeEnabled
  // TODO: 에디터 설정 트래킹

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Success) {
      model.initializeAiOptIn(model.query.data.me.preferences.aiOptIn())
    }
  }

  val aiOptIn = model.aiOptIn
  var showAiOptInSheet by remember { mutableStateOf(false) }

  LaunchedEffect(showAiOptInSheet) {
    if (!showAiOptInSheet) return@LaunchedEffect

    showAiOptInSheet = false
    bottomSheetHost.show {
      AiOptInSheet(
        isSubmitting = model.isUpdatingAiOptIn,
        onConfirm = {
          model.updateAiOptIn(true) {
            dismiss()
          }
        },
      )
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("에디터 설정", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
  ) { contentPadding ->
    Column(
      modifier = Modifier
        .fillMaxSize()
        .verticalScroll(scrollState)
        .padding(contentPadding)
        .navigationBarsPadding(),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text(
        "에디터 설정",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      EditorSettingsSection(title = "작성 위치") {
        SettingControlRow(
          label = "타자기 모드",
          description = "현재 작성 중인 줄을 항상 화면의 특정 위치에 고정합니다.",
          onClick = {
            val next = !typewriterEnabled
            editorPreferences.typewriterEnabled = next
          },
          trailing = {
            SettingSwitch(
              checked = typewriterEnabled,
              onCheckedChange = { next ->
                editorPreferences.typewriterEnabled = next
              },
            )
          },
        )

        if (typewriterEnabled) {
          CardDivider(inset = 20.dp)
          Column(
            modifier = Modifier
              .fillMaxWidth()
              .padding(horizontal = 20.dp, vertical = 18.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
          ) {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
              Text("고정 위치", style = AppTheme.typography.label)
              Text(
                "현재 작성 중인 줄이 고정될 화면상의 위치를 설정합니다.",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
              )
            }

            Row(
              verticalAlignment = Alignment.CenterVertically,
              horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
              Text(
                "화면 상단",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
              )
              QuietSlider(
                value = typewriterPosition,
                modifier = Modifier.weight(1f),
                onValueChange = { next ->
                  editorPreferences.typewriterPosition = next
                },
              )
              Text(
                "화면 하단",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
              )
            }
          }
        }
      }

      EditorSettingsSection(title = "표시 설정") {
        SettingControlRow(
          label = "현재 줄 강조",
          description = "현재 작성 중인 줄을 강조하여 화면에 표시합니다.",
          onClick = {
            val next = !lineHighlightEnabled
            editorPreferences.lineHighlightEnabled = next
          },
          trailing = {
            SettingSwitch(
              checked = lineHighlightEnabled,
              onCheckedChange = { next ->
                editorPreferences.lineHighlightEnabled = next
              },
            )
          },
        )
      }

      EditorSettingsSection(title = "편집 설정") {
        SettingControlRow(
          label = "선택 영역 둘러싸기",
          description = "따옴표나 괄호를 입력하면 선택 영역을 둘러쌉니다.",
          onClick = {
            val next = !autoSurroundEnabled
            editorPreferences.autoSurroundEnabled = next
          },
          trailing = {
            SettingSwitch(
              checked = autoSurroundEnabled,
              onCheckedChange = { next ->
                editorPreferences.autoSurroundEnabled = next
              },
            )
          },
        )
      }

      EditorSettingsSection(title = "위젯 설정") {
        SettingControlRow(
          label = "글자 수 위젯",
          description = "에디터에서 글자 수를 표시합니다.",
          onClick = {
            val next = !characterCountFloatingEnabled
            editorPreferences.characterCountFloatingEnabled = next
          },
          trailing = {
            SettingSwitch(
              checked = characterCountFloatingEnabled,
              onCheckedChange = { next ->
                editorPreferences.characterCountFloatingEnabled = next
              },
            )
          },
        )

        if (characterCountFloatingEnabled) {
          CardDivider(inset = 20.dp)
          SettingControlRow(
            label = "위젯 자동 페이드 인/아웃",
            description = "타이핑, 스크롤 시 위젯이 잠시 사라집니다.",
            onClick = {
              val next = !widgetAutoFadeEnabled
              editorPreferences.widgetAutoFadeEnabled = next
            },
            trailing = {
              SettingSwitch(
                checked = widgetAutoFadeEnabled,
                onCheckedChange = { next ->
                  editorPreferences.widgetAutoFadeEnabled = next
                },
              )
            },
          )
        }
      }

      EditorSettingsSection(title = "AI 설정") {
        SettingControlRow(
          label = "AI 기능 활성화",
          description = "활성화하면 AI 피드백 등 타이피가 제공하는 AI 기능을 사용할 수 있어요.",
          enabled = !model.isUpdatingAiOptIn,
          onClick = {
            if (aiOptIn) {
              model.updateAiOptIn(false)
            } else {
              showAiOptInSheet = true
            }
          },
          trailing = {
            SettingSwitch(
              checked = aiOptIn,
              enabled = !model.isUpdatingAiOptIn,
              onCheckedChange = { next ->
                if (next) {
                  showAiOptInSheet = true
                } else {
                  model.updateAiOptIn(false)
                }
              },
            )
          },
        )
      }

      Spacer(Modifier.height(72.dp))
    }
  }
}

@KoinViewModel
class EditorSettingsViewModel(
  private val loader: Loader,
  private val toast: Toast,
) : GraphQLViewModel() {
  val query = watchQuery(placeholderData()) { EditorSettingsScreen_Query() }

  var aiOptIn by mutableStateOf(false)
    private set
  private var hasInitializedAiOptIn by mutableStateOf(false)

  var isUpdatingAiOptIn by mutableStateOf(false)
    private set

  fun initializeAiOptIn(enabled: Boolean) {
    if (!hasInitializedAiOptIn) {
      aiOptIn = enabled
      hasInitializedAiOptIn = true
    }
  }

  fun updateAiOptIn(enabled: Boolean, onSuccess: (() -> Unit)? = null) {
    if (isUpdatingAiOptIn) return

    viewModelScope.launch {
      isUpdatingAiOptIn = true
      try {
        loader.runWith {
          executeMutation(
            EditorSettingsScreen_UpdatePreferences_Mutation(
              input = UpdatePreferencesInput(
                value = JsonObject(mapOf("aiOptIn" to JsonPrimitive(enabled))),
              ),
            ),
          )
        }
        aiOptIn = enabled
        query.refetch()
        onSuccess?.invoke()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        Logger.e(e) { "Failed to update aiOptIn" }
        toast.show(ToastType.Error, "오류가 발생했어요. 잠시 후 다시 시도해주세요.")
      } finally {
        isUpdatingAiOptIn = false
      }
    }
  }
}

@Composable
private fun EditorSettingsSection(
  title: String,
  content: @Composable ColumnScope.() -> Unit,
) {
  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    SectionTitle(
      title,
      modifier = Modifier.padding(top = 4.dp),
    )

    CardSurface(
      modifier = Modifier.fillMaxWidth(),
    ) {
      Column(content = content)
    }
  }
}

@Composable
private fun QuietSlider(
  value: Double,
  onValueChange: (Double) -> Unit,
  modifier: Modifier = Modifier,
) {
  val colors = AppTheme.colors
  val haptic = LocalHapticFeedback.current
  BoxWithConstraints(
    modifier = modifier
      .height(32.dp),
    contentAlignment = Alignment.CenterStart,
  ) {
    val density = LocalDensity.current
    val thumbSize = 24.dp
    val travel = (maxWidth - thumbSize).coerceAtLeast(0.dp)
    val travelPx = with(density) { travel.toPx() }
    val sliderWidthPx = with(density) { maxWidth.toPx() }
    val thumbRadiusPx = with(density) { (thumbSize / 2).toPx() }
    val onValueChangeState by rememberUpdatedState(onValueChange)
    val hapticState by rememberUpdatedState(haptic)
    val thumbOffset = travel * value.toFloat().coerceIn(0f, 1f)
    val filledFraction = value.toFloat().coerceIn(0f, 1f)

    fun snap(raw: Float): Double {
      val stepped = (raw.coerceIn(0f, 1f) / 0.05f).roundToInt() * 0.05f
      return stepped.coerceIn(0f, 1f).toDouble()
    }

    fun valueAtTouch(x: Float): Double {
      if (travelPx <= 0f || sliderWidthPx <= 0f) return 0.0
      val fraction = ((x - thumbRadiusPx) / travelPx).coerceIn(0f, 1f)
      return snap(fraction)
    }

    Box(
      modifier = Modifier
        .fillMaxWidth()
        .height(8.dp)
        .background(colors.borderStrong.copy(alpha = 0.55f), CircleShape),
    ) {
      Box(
        modifier = Modifier
          .fillMaxWidth(filledFraction)
          .height(8.dp)
          .background(colors.brand.copy(alpha = 0.72f), CircleShape),
      )
    }

    Box(
      modifier = Modifier
        .matchParentSize()
        .pointerInput(maxWidth) {
          awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false)
            var gestureValue = value

            fun updateGestureValue(x: Float) {
              val next = valueAtTouch(x)
              if (next == gestureValue) return
              gestureValue = next
              hapticState.performHapticFeedback(HapticFeedbackType.SegmentTick)
              onValueChangeState(next)
            }

            updateGestureValue(down.position.x)

            while (true) {
              val event = awaitPointerEvent()
              val change = event.changes.firstOrNull { it.id == down.id } ?: break
              if (!change.pressed) {
                break
              }
              updateGestureValue(change.position.x)
              change.consume()
            }
          }
        },
    )

    Box(
      modifier = Modifier
        .offset(x = thumbOffset)
        .size(thumbSize)
        .dropShadow(CircleShape) {
          color = colors.shadowAmbient
          radius = 4f
        }
        .dropShadow(CircleShape) {
          color = colors.shadow
          radius = 8f
          offset = Offset(0f, 1f)
        }
        .background(colors.surfaceRaised, CircleShape)
        .border(1.dp, colors.borderDefault, CircleShape),
    )
  }
}

@Composable
private fun BottomSheetScope<Unit>.AiOptInSheet(
  isSubmitting: Boolean,
  onConfirm: suspend () -> Unit,
) {
  Column(
    modifier = Modifier
      .fillMaxWidth()
      .padding(horizontal = 16.dp),
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    Text("AI 기능을 활성화하시겠어요?", style = AppTheme.typography.title)

    Text(
      "타이피는 사용자의 프라이버시를 최우선으로 생각해요. 사용자가 작성한 글은 어떠한 경우에도 AI 모델 학습에 사용되지 않아요.",
      style = AppTheme.typography.body,
      color = AppTheme.colors.textSecondary,
    )

    CardSurface(
      modifier = Modifier.fillMaxWidth(),
    ) {
      Column(
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
      ) {
        AiOptInNoticeItem(
          title = "학습 금지",
          description = "사용자의 글은 AI 모델 학습이나 개선에 절대 사용되지 않아요.",
        )
        AiOptInNoticeItem(
          title = "요청 시에만",
          description = "사용자가 요청하지 않는 한 타이피가 임의로 AI를 사용하지 않아요.",
        )
        AiOptInNoticeItem(
          title = "투명한 처리",
          description = "AI가 언제, 어떻게 사용되는지 사용자가 항상 알 수 있어요.",
        )
        AiOptInNoticeItem(
          title = "완전한 통제",
          description = "AI 기능은 언제든 끌 수 있고, 비활성화하면 어떤 AI 처리도 일어나지 않아요.",
        )
        AiOptInNoticeItem(
          title = "권리 보장",
          description = "타이피는 사용자 창작물에 대한 어떤 권리도 주장하지 않아요.",
        )
      }
    }

    Button(
      text = "활성화",
      enabled = !isSubmitting,
      loading = isSubmitting,
      onClick = onConfirm,
    )
  }
}

@Composable
private fun AiOptInNoticeItem(
  title: String,
  description: String,
) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.spacedBy(8.dp),
    verticalAlignment = Alignment.Top,
  ) {
    Text(
      "•",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
    )
    Text(
      buildAnnotatedString {
        withStyle(SpanStyle(fontWeight = FontWeight.W600)) {
          append("$title: ")
        }
        append(description)
      },
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textTertiary,
      modifier = Modifier.weight(1f),
    )
  }
}

private fun JsonElement.aiOptIn(): Boolean {
  val json = this as? JsonObject ?: return false
  return json["aiOptIn"]?.jsonPrimitive?.booleanOrNull ?: false
}

private fun placeholderData() = EditorSettingsScreen_Query.Data(PlaceholderResolver) {
  me = buildUser {
    preferences = JsonObject(emptyMap())
  }
}
