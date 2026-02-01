import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/native/editor_texture_renderer.dart';
import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/external/overlay.dart';

class PageItem extends HookWidget {
  const PageItem({
    required this.pageIndex,
    required this.editor,
    required this.renderVersion,
    required this.bottomGap,
    required this.pageWidth,
    required this.pageHeight,
    required this.cursorInfo,
    required this.isFocused,
    required this.onSelectionStart,
    required this.onSelectionEnd,
    required this.onTap,
    super.key,
  });

  final int pageIndex;
  final NativeEditor editor;
  final Object? renderVersion;
  final double bottomGap;
  final double pageWidth;
  final double? pageHeight;
  final CursorInfo? cursorInfo;
  final bool isFocused;
  final VoidCallback onSelectionStart;
  final VoidCallback onSelectionEnd;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final renderer = useRef<EditorTextureRenderer?>(null);
    final textureId = useState<int?>(null);
    final lastTapTime = useRef<DateTime?>(null);
    final lastTapPosition = useRef<Offset?>(null);

    useEffect(() {
      Future<void> initRenderer() async {
        renderer.value ??= EditorTextureRenderer(editor: editor);

        final r = renderer.value!;
        if (r.textureId == null) {
          await r.create(pageIndex);
        }

        if (r.textureId != null) {
          await r.render(pageIndex);
          textureId.value = r.textureId;
        }
      }

      unawaited(initRenderer());
      return null;
    }, [pageIndex, renderVersion]);

    useEffect(() {
      return () {
        unawaited(renderer.value?.dispose());
      };
    }, const []);

    final hasTexture = textureId.value != null && pageHeight != null;

    if (hasTexture) {
      return Padding(
        padding: EdgeInsets.only(bottom: bottomGap),
        child: GestureDetector(
          onTapDown: (details) {
            onTap();

            final now = DateTime.now();
            final prevTime = lastTapTime.value;
            final prevPosition = lastTapPosition.value;

            var clickCount = 1;
            if (prevTime != null && prevPosition != null) {
              final timeDiff = now.difference(prevTime).inMilliseconds;
              final distance = (details.localPosition - prevPosition).distance;
              if (timeDiff < 300 && distance < 20) {
                clickCount = 2;
              }
            }

            lastTapTime.value = now;
            lastTapPosition.value = details.localPosition;

            editor.dispatch({
              'type': 'pointerDown',
              'pageIdx': pageIndex,
              'x': details.localPosition.dx,
              'y': details.localPosition.dy,
              'clickCount': clickCount,
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
          child: SizedBox(
            width: pageWidth,
            height: pageHeight,
            child: Stack(
              children: [
                SizedBox.expand(child: Texture(textureId: textureId.value!)),
                EditorCursor(cursorInfo: cursorInfo, isFocused: isFocused),
                ExternalElementOverlay(pageIndex: pageIndex),
              ],
            ),
          ),
        ),
      );
    }

    return Container(
      height: pageHeight,
      margin: EdgeInsets.only(bottom: bottomGap),
      child: const Center(child: CircularProgressIndicator()),
    );
  }
}
