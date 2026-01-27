import 'dart:async';
import 'dart:convert';
import 'dart:ui' as ui;

import 'package:auto_route/auto_route.dart';
import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.data.gql.dart';
import 'package:typie/screens/native_editor/__generated__/native_editor_query.req.gql.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

import 'cursor.dart';
import 'theme.dart';

const _fontCdnBase = 'https://cdn.typie.net/fonts/editor';

@RoutePage()
class NativeEditorScreen extends StatelessWidget {
  const NativeEditorScreen({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceDefault,
      operation: GNativeEditorScreen_QueryReq((b) => b.vars.slug = slug),
      builder: (context, client, data) => _Content(data: data),
    );
  }
}

class _Content extends HookWidget {
  const _Content({required this.data});

  final GNativeEditorScreen_QueryData data;

  @override
  Widget build(BuildContext context) {
    final error = useState<String?>(null);
    final app = useRef<NativeEditorApplication?>(null);
    final editor = useState<NativeEditor?>(null);

    final document = data.entity.node.when(document: (doc) => doc, orElse: () => null);
    final title = document?.title ?? '(제목 없음)';
    final scaleFactor = ui.PlatformDispatcher.instance.views.first.devicePixelRatio;

    final brightness = MediaQuery.platformBrightnessOf(context);

    useEffect(() {
      if (document == null) {
        error.value = 'Document not found';
        return null;
      }

      final theme = getEditorTheme(brightness);

      Future<void> init() async {
        try {
          final snapshotBase64 = document.snapshot.value;
          final snapshot = snapshotBase64.isNotEmpty ? base64Decode(snapshotBase64) : null;

          app.value = await _initApplication();
          editor.value = app.value!.createEditor(scaleFactor, snapshot: snapshot)
            ..dispatch({
              'type': 'initialize',
              'theme': {'colors': theme},
            });
        } on EditorException catch (err) {
          error.value = err.message;
        } catch (err) {
          error.value = err.toString();
        }
      }

      unawaited(init());

      return () {
        editor.value?.dispose();
        app.value?.dispose();
      };
    }, [document?.id]);

    useEffect(() {
      final currentEditor = editor.value;
      if (currentEditor == null || currentEditor.isDisposed) {
        return null;
      }

      final theme = getEditorTheme(brightness);
      currentEditor.dispatch({
        'type': 'setTheme',
        'theme': {'colors': theme},
      });

      return null;
    }, [editor.value, brightness]);

    final isLoading = editor.value == null && error.value == null && document != null;

    return Screen(
      heading: Heading(title: title, backgroundColor: context.colors.surfaceDefault),
      backgroundColor: context.colors.surfaceDefault,
      keyboardDismiss: false,
      responsive: false,
      child: _buildBody(context, isLoading: isLoading, error: error.value, editor: editor.value),
    );
  }

  Widget _buildBody(
    BuildContext context, {
    required bool isLoading,
    required String? error,
    required NativeEditor? editor,
  }) {
    if (isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (error != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(LucideLightIcons.circle_alert, size: 48, color: context.colors.textSubtle),
              const SizedBox(height: 16),
              Text(
                '에디터를 불러올 수 없습니다',
                style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600, color: context.colors.textDefault),
              ),
              const SizedBox(height: 8),
              Text(
                error,
                style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      );
    }

    if (editor == null) {
      return const Center(child: CircularProgressIndicator());
    }

    return LayoutBuilder(
      builder: (context, constraints) {
        return _EditorView(editor: editor, width: constraints.maxWidth, height: constraints.maxHeight);
      },
    );
  }
}

const _pageGap = 24.0;

class _EditorInput extends StatefulWidget {
  const _EditorInput({required this.editor, required this.focusNode, required this.child, super.key});

  final NativeEditor editor;
  final FocusNode focusNode;
  final Widget child;

  @override
  State<_EditorInput> createState() => _EditorInputState();
}

class _EditorInputState extends State<_EditorInput> implements TextInputClient {
  static const _sentinel = '\u200B';
  static const _sentinelValue = TextEditingValue(text: _sentinel, selection: TextSelection.collapsed(offset: 1));

  TextInputConnection? _connection;
  TextEditingValue _currentValue = _sentinelValue;
  bool _isComposing = false;
  String _composingText = '';
  int _committedLength = 1;
  Timer? _deferTimer;

  @override
  void initState() {
    super.initState();
    widget.focusNode.addListener(_onFocusChanged);
    HardwareKeyboard.instance.addHandler(_onKeyEvent);
  }

  @override
  void dispose() {
    HardwareKeyboard.instance.removeHandler(_onKeyEvent);
    _deferTimer?.cancel();
    widget.focusNode.removeListener(_onFocusChanged);
    _closeConnection();
    super.dispose();
  }

  bool _onKeyEvent(KeyEvent event) {
    if (_connection == null || !_connection!.attached) {
      return false;
    }

    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return false;
    }

    final message = _getActionFromKeyEvent(event);
    if (message == null) {
      return false;
    }

    _commitComposing();
    widget.editor.dispatch(message);
    _resetState();
    return true;
  }

  void _onFocusChanged() {
    if (widget.focusNode.hasFocus) {
      _openConnection();
    } else {
      _closeConnection();
    }
  }

  void _openConnection() {
    if (_connection != null && _connection!.attached) {
      return;
    }

    _deferTimer?.cancel();
    _connection = TextInput.attach(
      this,
      const TextInputConfiguration(inputType: TextInputType.multiline, inputAction: TextInputAction.newline),
    );
    _connection!.show();
    _resetState();
  }

  void _closeConnection() {
    _deferTimer?.cancel();
    _connection?.close();
    _connection = null;
    _commitComposing();
  }

  void open() {
    widget.focusNode.requestFocus();
    _openConnection();
  }

  @override
  void updateEditingValue(TextEditingValue value) {
    _deferTimer?.cancel();
    _deferTimer = null;
    _currentValue = value;

    final newText = value.text;

    if (newText.isEmpty) {
      if (_isComposing) {
        _deferCancelComposing();
      } else {
        widget.editor.dispatch({'type': 'deleteBackward'});
        _resetState();
      }
      return;
    }

    final uncommitted = newText.substring(_committedLength.clamp(0, newText.length));

    if (uncommitted.isEmpty) {
      if (_isComposing) {
        _deferCancelComposing();
      }
      return;
    }

    if (uncommitted.contains('\n')) {
      _commitComposing();
      _resetState();
      return;
    }

    final composing = _isComposingText(value, uncommitted);

    if (uncommitted.length == 1 && composing) {
      _composingText = uncommitted;
      if (!_isComposing) {
        _isComposing = true;
        widget.editor.dispatch({'type': 'compositionStart', 'text': uncommitted});
      } else {
        widget.editor.dispatch({'type': 'compositionUpdate', 'text': uncommitted});
      }
      return;
    }

    if (_isComposing) {
      final lastChar = uncommitted[uncommitted.length - 1];
      final prefix = uncommitted.substring(0, uncommitted.length - 1);

      if (_isComposingText(value, lastChar)) {
        widget.editor.dispatch({'type': 'input', 'text': prefix});
        widget.editor.dispatch({'type': 'compositionEnd'});
        _committedLength += prefix.length;
        _isComposing = true;
        _composingText = lastChar;
        widget.editor.dispatch({'type': 'compositionStart', 'text': lastChar});
      } else {
        widget.editor.dispatch({'type': 'input', 'text': uncommitted});
        widget.editor.dispatch({'type': 'compositionEnd'});
        _isComposing = false;
        _composingText = '';
        _resetState();
      }
      return;
    }

    widget.editor.dispatch({'type': 'input', 'text': uncommitted});
    _resetState();
  }

  void _commitComposing() {
    _deferTimer?.cancel();
    _deferTimer = null;
    if (_isComposing) {
      widget.editor.dispatch({'type': 'input', 'text': _composingText});
      widget.editor.dispatch({'type': 'compositionEnd'});
      _isComposing = false;
      _composingText = '';
    }
  }

  void _deferCancelComposing() {
    _deferTimer = Timer(Duration.zero, () {
      if (_isComposing) {
        widget.editor.dispatch({'type': 'compositionUpdate', 'text': ''});
        widget.editor.dispatch({'type': 'compositionEnd'});
        _isComposing = false;
        _composingText = '';
      }
      _resetState();
    });
  }

  void _resetState() {
    _currentValue = _sentinelValue;
    _isComposing = false;
    _composingText = '';
    _committedLength = 1;
    _connection?.setEditingState(_sentinelValue);
  }

  bool _isComposingText(TextEditingValue value, String text) {
    if (value.composing != TextRange.empty) {
      return true;
    }
    if (text.isEmpty) {
      return false;
    }
    return text.codeUnitAt(text.length - 1) > 0x7F;
  }

  @override
  void performAction(TextInputAction action) {
    if (action == TextInputAction.newline) {
      _commitComposing();
      widget.editor.dispatch({'type': 'insertNewline'});
      _resetState();
    }
  }

  @override
  void performPrivateCommand(String action, Map<String, dynamic> data) {}

  @override
  void showAutocorrectionPromptRect(int start, int end) {}

  @override
  void updateFloatingCursor(RawFloatingCursorPoint point) {}

  @override
  void connectionClosed() {
    _commitComposing();
    _connection = null;
  }

  @override
  AutofillScope? get currentAutofillScope => null;

  @override
  TextEditingValue? get currentTextEditingValue => _currentValue;

  @override
  void insertContent(KeyboardInsertedContent content) {}

  @override
  void didChangeInputControl(TextInputControl? oldControl, TextInputControl? newControl) {}

  @override
  void insertTextPlaceholder(Size size) {}

  @override
  void removeTextPlaceholder() {}

  @override
  void showToolbar() {}

  @override
  void performSelector(String selectorName) {}

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}

class _EditorView extends HookWidget {
  const _EditorView({required this.editor, required this.width, required this.height});

  final NativeEditor editor;
  final double width;
  final double height;

  @override
  Widget build(BuildContext context) {
    final layout = useState<_LayoutInfo?>(null);
    final renderVersion = useState<Object>(Object());
    final cursorInfo = useState<CursorInfo?>(null);
    final isFocused = useState(false);
    final lastSize = useRef<(double, double, double)?>(null);
    final tickerProvider = useSingleTickerProvider();
    final focusNode = useFocusNode();
    final inputKey = useMemoized(GlobalKey<_EditorInputState>.new);

    useEffect(() {
      void onTick(Duration elapsed) {
        if (editor.isDisposed) {
          return;
        }

        final scaleFactor = ui.PlatformDispatcher.instance.views.first.devicePixelRatio;
        final currentSize = (width, height, scaleFactor);

        if (lastSize.value != currentSize) {
          lastSize.value = currentSize;
          editor.dispatch({'type': 'resize', 'width': width, 'height': height, 'scaleFactor': scaleFactor});
        }

        final cmds = editor.tick();
        if (cmds != null) {
          for (final cmd in cmds) {
            switch (cmd) {
              case {
                'type': 'layoutChanged',
                'pageCount': final int pageCount,
                'layoutMode': final Map<String, dynamic> layoutMode,
                'pageHeights': final List<dynamic> pageHeights,
              }:
                layout.value = _LayoutInfo(
                  pageCount: pageCount,
                  isPaginated: layoutMode['type'] == 'paginated',
                  pageHeights: pageHeights.cast<num>().map((e) => e.toDouble()).toList(),
                );
              case {'type': 'renderRequired'}:
                renderVersion.value = Object();
              case {'type': 'cursorChanged'}:
                cursorInfo.value = CursorInfo.fromMap(cmd as Map<String, dynamic>);
            }
          }
        }

        unawaited(
          SchedulerBinding.instance.scheduleTask(() {
            if (!editor.isDisposed) {
              editor.flush();
            }
          }, Priority.idle),
        );
      }

      final ticker = tickerProvider.createTicker(onTick);
      unawaited(ticker.start());

      return ticker.dispose;
    }, []);

    final isSelecting = useState(false);

    final currentLayout = layout.value;
    if (currentLayout == null) {
      return const Center(child: CircularProgressIndicator());
    }

    return Focus(
      focusNode: focusNode,
      onFocusChange: (focused) => isFocused.value = focused,
      child: _EditorInput(
        key: inputKey,
        editor: editor,
        focusNode: focusNode,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: () => inputKey.currentState?.open(),
          child: ListView.builder(
            itemCount: currentLayout.pageCount,
            cacheExtent: 1000,
            physics: isSelecting.value ? const NeverScrollableScrollPhysics() : null,
            itemBuilder: (context, index) {
              final isLast = index == currentLayout.pageCount - 1;
              final gap = currentLayout.isPaginated && !isLast ? _pageGap : 0.0;
              final pageHeight = currentLayout.pageHeights.elementAtOrNull(index);
              final pageCursor = cursorInfo.value?.pageIdx == index ? cursorInfo.value : null;
              return _PageItem(
                key: ValueKey(index),
                pageIndex: index,
                editor: editor,
                renderVersion: renderVersion.value,
                bottomGap: gap,
                placeholderHeight: pageHeight,
                cursorInfo: pageCursor,
                isFocused: isFocused.value,
                onSelectionStart: () => isSelecting.value = true,
                onSelectionEnd: () => isSelecting.value = false,
                onTap: () => inputKey.currentState?.open(),
              );
            },
          ),
        ),
      ),
    );
  }
}

class _LayoutInfo {
  const _LayoutInfo({required this.pageCount, required this.isPaginated, required this.pageHeights});

  final int pageCount;
  final bool isPaginated;
  final List<double> pageHeights;
}

class _PageItem extends HookWidget {
  const _PageItem({
    required this.pageIndex,
    required this.editor,
    required this.renderVersion,
    required this.bottomGap,
    required this.placeholderHeight,
    required this.cursorInfo,
    required this.isFocused,
    required this.onSelectionStart,
    required this.onSelectionEnd,
    required this.onTap,
    super.key,
  });

  final int pageIndex;
  final NativeEditor editor;
  final Object renderVersion;
  final double bottomGap;
  final double? placeholderHeight;
  final CursorInfo? cursorInfo;
  final bool isFocused;
  final VoidCallback onSelectionStart;
  final VoidCallback onSelectionEnd;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final image = useState<ui.Image?>(null);

    useEffect(() {
      Future<void> render() async {
        image.value = await _renderPage(editor, pageIndex);
      }

      unawaited(render());
      return null;
    }, [pageIndex, renderVersion]);

    if (image.value != null) {
      return Padding(
        padding: EdgeInsets.only(bottom: bottomGap),
        child: GestureDetector(
          onTapDown: (details) {
            onTap();
            editor.dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIndex,
              'x': details.localPosition.dx,
              'y': details.localPosition.dy,
              'clickCount': 1,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
          },
          onTapUp: (details) {
            editor.dispatch({
              'type': 'pointerUp',
              'pageIdx': pageIndex,
              'x': details.localPosition.dx,
              'y': details.localPosition.dy,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
          },
          onLongPressStart: (details) {
            onSelectionStart();
            editor.dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIndex,
              'x': details.localPosition.dx,
              'y': details.localPosition.dy,
              'clickCount': 1,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
          },
          onLongPressMoveUpdate: (details) {
            editor.dispatch({
              'type': 'pointerMove',
              'pageIdx': pageIndex,
              'x': details.localPosition.dx,
              'y': details.localPosition.dy,
              'buttons': 1,
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
          },
          onLongPressEnd: (details) {
            editor.dispatch({
              'type': 'pointerUp',
              'pageIdx': pageIndex,
              'x': details.localPosition.dx,
              'y': details.localPosition.dy,
              'button': 'primary',
              'modifier': {'shift': false, 'ctrl': false, 'alt': false, 'meta': false},
            });
            onSelectionEnd();
          },
          child: Stack(
            children: [
              RawImage(image: image.value),
              EditorCursor(cursorInfo: cursorInfo, isFocused: isFocused),
            ],
          ),
        ),
      );
    }

    return Container(
      height: placeholderHeight,
      margin: EdgeInsets.only(bottom: bottomGap),
      child: const Center(child: CircularProgressIndicator()),
    );
  }
}

Map<String, dynamic> _nav(String direction, bool extend) => {
  'type': 'navigate',
  'direction': direction,
  'extend': extend,
};

Map<String, dynamic>? _getActionFromKeyEvent(KeyEvent event) {
  final key = event.logicalKey;
  final shift = HardwareKeyboard.instance.isShiftPressed;
  final meta = HardwareKeyboard.instance.isMetaPressed;
  final ctrl = HardwareKeyboard.instance.isControlPressed;
  final alt = HardwareKeyboard.instance.isAltPressed;

  final wordModifier = defaultTargetPlatform == TargetPlatform.iOS ? alt : ctrl;
  final actionModifier = defaultTargetPlatform == TargetPlatform.iOS ? meta : ctrl;

  if (key == LogicalKeyboardKey.arrowLeft) {
    if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
      return _nav('lineStart', shift);
    } else if (wordModifier) {
      return _nav('wordLeft', shift);
    } else {
      return _nav('left', shift);
    }
  } else if (key == LogicalKeyboardKey.arrowRight) {
    if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
      return _nav('lineEnd', shift);
    } else if (wordModifier) {
      return _nav('wordRight', shift);
    } else {
      return _nav('right', shift);
    }
  } else if (key == LogicalKeyboardKey.arrowUp) {
    if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
      return _nav('documentStart', shift);
    } else {
      return _nav('up', shift);
    }
  } else if (key == LogicalKeyboardKey.arrowDown) {
    if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
      return _nav('documentEnd', shift);
    } else {
      return _nav('down', shift);
    }
  } else if (key == LogicalKeyboardKey.home) {
    if (ctrl) {
      return _nav('documentStart', shift);
    } else {
      return _nav('lineStart', shift);
    }
  } else if (key == LogicalKeyboardKey.end) {
    if (ctrl) {
      return _nav('documentEnd', shift);
    } else {
      return _nav('lineEnd', shift);
    }
  } else if (key == LogicalKeyboardKey.pageUp) {
    return _nav('pageUp', shift);
  } else if (key == LogicalKeyboardKey.pageDown) {
    return _nav('pageDown', shift);
  } else if (key == LogicalKeyboardKey.backspace) {
    if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
      return {'type': 'deleteToLineStart'};
    } else if (wordModifier) {
      return {'type': 'deleteWordBackward'};
    } else {
      return {'type': 'deleteBackward'};
    }
  } else if (key == LogicalKeyboardKey.delete) {
    if (wordModifier) {
      return {'type': 'deleteWordForward'};
    } else {
      return {'type': 'deleteForward'};
    }
  } else if (key == LogicalKeyboardKey.enter || key == LogicalKeyboardKey.numpadEnter) {
    if (actionModifier) {
      return {'type': 'insertPageBreak'};
    } else if (shift) {
      return {'type': 'insertHardBreak'};
    } else {
      return {'type': 'insertNewline'};
    }
  } else if (key == LogicalKeyboardKey.keyA && actionModifier) {
    return {'type': 'selectAll'};
  } else if (key == LogicalKeyboardKey.keyB && actionModifier) {
    return {'type': 'toggleBold'};
  } else if (key == LogicalKeyboardKey.keyI && actionModifier) {
    return {'type': 'toggleItalic'};
  } else if (key == LogicalKeyboardKey.keyU && actionModifier) {
    return {'type': 'toggleUnderline'};
  } else if (key == LogicalKeyboardKey.keyS && shift && actionModifier) {
    return {'type': 'toggleStrikethrough'};
  } else if (key == LogicalKeyboardKey.keyZ && actionModifier) {
    if (shift) {
      return {'type': 'redo'};
    } else {
      return {'type': 'undo'};
    }
  } else if (key == LogicalKeyboardKey.backslash && actionModifier) {
    return {'type': 'clearFormatting'};
  } else if (key == LogicalKeyboardKey.tab) {
    if (shift) {
      return {'type': 'outdent'};
    } else {
      return {'type': 'indent'};
    }
  } else if (key == LogicalKeyboardKey.escape) {
    return {'type': 'escape'};
  }

  return null;
}

Future<NativeEditorApplication> _initApplication() async {
  final icuData = await rootBundle.load('assets/native/icu_data.postcard');
  final fontResponse = await Dio().get<List<int>>(
    '$_fontCdnBase/Pretendard-Regular.ttf',
    options: Options(responseType: ResponseType.bytes),
  );

  return NativeEditorApplication()
    ..loadIcuData(icuData.buffer.asUint8List())
    ..registerFont('Pretendard', 400, Uint8List.fromList(fontResponse.data!))
    ..setAvailableFonts({
      'Pretendard': [400, 500, 600, 700],
    });
}

Future<ui.Image> _renderPage(NativeEditor editor, int pageIndex) async {
  final result = editor.renderPage(pageIndex);
  final buffer = await ui.ImmutableBuffer.fromUint8List(result.data);
  final descriptor = ui.ImageDescriptor.raw(
    buffer,
    width: result.width,
    height: result.height,
    pixelFormat: ui.PixelFormat.rgba8888,
  );
  final codec = await descriptor.instantiateCodec();
  final frame = await codec.getNextFrame();
  return frame.image;
}
