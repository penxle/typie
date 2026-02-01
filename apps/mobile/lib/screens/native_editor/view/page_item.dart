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
    required this.lineHighlightEnabled,
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
  final bool lineHighlightEnabled;

  @override
  Widget build(BuildContext context) {
    final renderer = useRef<EditorTextureRenderer?>(null);
    final textureId = useState<int?>(null);
    final textureSize = useState<Size?>(null);
    final isMounted = useRef(true);

    final devicePixelRatio = MediaQuery.devicePixelRatioOf(context);

    useEffect(() {
      Future<void> initRenderer() async {
        renderer.value ??= EditorTextureRenderer(editor: editor);

        final r = renderer.value!;
        if (r.textureId == null) {
          await r.create(pageIndex);
        }

        if (!isMounted.value) {
          return;
        }

        if (r.textureId != null) {
          await r.render(pageIndex);
          if (!isMounted.value) {
            return;
          }
          textureId.value = r.textureId;
          textureSize.value = Size(r.width / devicePixelRatio, r.height / devicePixelRatio);
        }
      }

      unawaited(initRenderer());
      return null;
    }, [pageIndex, renderVersion]);

    useEffect(() {
      return () {
        isMounted.value = false;
        unawaited(renderer.value?.dispose());
      };
    }, const []);

    final hasTexture = textureId.value != null && textureSize.value != null;

    if (hasTexture) {
      return Padding(
        padding: EdgeInsets.only(bottom: bottomGap),
        child: SizedBox.fromSize(
          size: textureSize.value,
          child: Stack(
            children: [
              LineHighlight(cursorInfo: cursorInfo, isFocused: isFocused, enabled: lineHighlightEnabled),
              SizedBox.expand(child: Texture(textureId: textureId.value!)),
              EditorCursor(cursorInfo: cursorInfo, isFocused: isFocused),
              ExternalElementOverlay(pageIndex: pageIndex),
            ],
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
