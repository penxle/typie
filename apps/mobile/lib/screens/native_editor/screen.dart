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
import 'package:typie/screens/native_editor/editor_input_view.dart';
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
    final inputKey = useMemoized(GlobalKey<EditorInputViewState>.new);
    final isActive = useRef(false);
    final inputCausedCursorChange = useRef(false);

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

    useEffect(() {
      final cursor = cursorInfo.value;
      if (cursor != null && cursor.show) {
        inputKey.currentState?.updateCursor(cursor.x, cursor.y, cursor.height);
      }
      if (inputCausedCursorChange.value) {
        inputCausedCursorChange.value = false;
      } else {
        inputKey.currentState?.resetInputContext();
      }
      return null;
    }, [cursorInfo.value]);

    useEffect(() {
      bool onKeyEvent(KeyEvent event) {
        if (!isActive.value) {
          return false;
        }
        if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
          return false;
        }
        final message = _getActionFromKeyEvent(event);
        if (message == null) {
          return false;
        }
        editor.dispatch(message);
        return true;
      }

      HardwareKeyboard.instance.addHandler(onKeyEvent);
      return () => HardwareKeyboard.instance.removeHandler(onKeyEvent);
    }, []);

    final currentLayout = layout.value;
    if (currentLayout == null) {
      return const Center(child: CircularProgressIndicator());
    }

    void openInput() {
      if (!isActive.value) {
        isActive.value = true;
        isFocused.value = true;
        inputKey.currentState?.activateInput();
      }
    }

    return Stack(
      children: [
        GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: openInput,
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
                onTap: openInput,
              );
            },
          ),
        ),
        Positioned.fill(
          child: EditorInputView(
            key: inputKey,
            onInsertText: (text) {
              inputCausedCursorChange.value = true;
              editor.dispatch({'type': 'input', 'text': text});
            },
            onDeleteBackward: () {
              inputCausedCursorChange.value = true;
              editor.dispatch({'type': 'deleteBackward'});
            },
            onSetMarkedText: (text) {
              inputCausedCursorChange.value = true;
            },
            onUnmarkText: () {
              inputCausedCursorChange.value = true;
            },
            onPerformAction: (action) {
              if (action == 'newline') {
                editor.dispatch({'type': 'insertNewline'});
              }
            },
            onShortcut: (action) {
              editor.dispatch({'type': action});
            },
          ),
        ),
      ],
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

  final physical = event.physicalKey;

  if (key == LogicalKeyboardKey.arrowLeft || physical == PhysicalKeyboardKey.arrowLeft) {
    if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
      return _nav('lineStart', shift);
    } else if (wordModifier) {
      return _nav('wordLeft', shift);
    } else {
      return _nav('left', shift);
    }
  } else if (key == LogicalKeyboardKey.arrowRight || physical == PhysicalKeyboardKey.arrowRight) {
    if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
      return _nav('lineEnd', shift);
    } else if (wordModifier) {
      return _nav('wordRight', shift);
    } else {
      return _nav('right', shift);
    }
  } else if (key == LogicalKeyboardKey.arrowUp || physical == PhysicalKeyboardKey.arrowUp) {
    if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
      return _nav('documentStart', shift);
    } else {
      return _nav('up', shift);
    }
  } else if (key == LogicalKeyboardKey.arrowDown || physical == PhysicalKeyboardKey.arrowDown) {
    if (defaultTargetPlatform == TargetPlatform.iOS && meta) {
      return _nav('documentEnd', shift);
    } else {
      return _nav('down', shift);
    }
  } else if (key == LogicalKeyboardKey.home || physical == PhysicalKeyboardKey.home) {
    if (ctrl) {
      return _nav('documentStart', shift);
    } else {
      return _nav('lineStart', shift);
    }
  } else if (key == LogicalKeyboardKey.end || physical == PhysicalKeyboardKey.end) {
    if (ctrl) {
      return _nav('documentEnd', shift);
    } else {
      return _nav('lineEnd', shift);
    }
  } else if (key == LogicalKeyboardKey.pageUp || physical == PhysicalKeyboardKey.pageUp) {
    return _nav('pageUp', shift);
  } else if (key == LogicalKeyboardKey.pageDown || physical == PhysicalKeyboardKey.pageDown) {
    return _nav('pageDown', shift);
  } else if (key == LogicalKeyboardKey.delete || physical == PhysicalKeyboardKey.delete) {
    if (wordModifier) {
      return {'type': 'deleteWordForward'};
    } else {
      return {'type': 'deleteForward'};
    }
  } else if (key == LogicalKeyboardKey.escape || physical == PhysicalKeyboardKey.escape) {
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
