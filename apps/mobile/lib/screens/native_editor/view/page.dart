import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/native/editor_texture_renderer.dart';
import 'package:typie/screens/native_editor/external/overlay.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/cursor.dart';
import 'package:typie/screens/native_editor/view/geometry.dart';
import 'package:typie/screens/native_editor/view/line_highlight.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/services/preference.dart';

const _cropMarkerSize = 32.0;

class PageItem extends HookWidget {
  const PageItem({required this.pageIndex, super.key});

  final int pageIndex;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final pref = useService<Pref>();
    final state = useListenable(scope.controller);

    final layout = state.state.layout!;
    final cursor = state.state.cursor;
    final pageCursor = cursor?.pageIdx == pageIndex ? cursor : null;
    final isFocused = state.state.isFocused;
    final layoutMode = layout.layoutMode;
    final margins = layoutMode is PaginatedLayoutMode ? layoutMode : null;
    final bottomGap = layout.isPaginated && pageIndex < layout.pageCount - 1 ? ContentGeometry.pageGap : 0.0;
    final pageHeight = layout.pageHeights.elementAtOrNull(pageIndex);

    final editor = scope.editor;
    final renderVersion = state.state.renderVersion;
    final lineHighlightEnabled = pref.lineHighlightEnabled;

    final renderer = useRef<EditorTextureRenderer?>(null);
    final textureId = useState<int?>(null);
    final textureSize = useState<Size?>(null);
    final isMounted = useRef(true);
    final displayCursor = useState<CursorInfo?>(pageCursor);
    final renderInProgress = useRef(false);

    final devicePixelRatio = MediaQuery.devicePixelRatioOf(context);

    Future<void> render() async {
      renderer.value ??= EditorTextureRenderer(editor: editor);
      final r = renderer.value!;

      if (r.textureId == null) {
        await r.create(pageIndex);
        if (!isMounted.value) {
          return;
        }
      }
      if (r.textureId == null) {
        return;
      }

      await r.render(pageIndex);
      if (!isMounted.value) {
        return;
      }

      textureId.value = r.textureId;
      textureSize.value = Size(r.width / devicePixelRatio, r.height / devicePixelRatio);
      renderInProgress.value = false;
      displayCursor.value = pageCursor;

      final pending = scope.pendingScroll.value;
      if (pending != null) {
        scope.pendingScroll.value = null;
        pending();
      }
    }

    useEffect(() {
      final timer = Timer(const Duration(milliseconds: 150), () {
        unawaited(render());
      });
      return () {
        timer.cancel();
        isMounted.value = false;
        unawaited(renderer.value?.dispose());
      };
    }, const []);

    useEffect(() {
      if (renderer.value?.textureId != null) {
        renderInProgress.value = true;
        unawaited(render());
      }
      return null;
    }, [renderVersion]);

    useEffect(() {
      if (!renderInProgress.value) {
        displayCursor.value = pageCursor;
      }
      return null;
    }, [pageCursor]);

    final hasTexture = textureId.value != null && textureSize.value != null;

    final pageDecoration = layout.isPaginated
        ? BoxDecoration(
            color: context.colors.surfaceDefault,
            boxShadow: [
              BoxShadow(
                color: context.colors.shadowDefault.withValues(alpha: 0.1),
                blurRadius: 8,
                offset: const Offset(0, 2),
              ),
            ],
            border: Border.all(color: context.colors.borderSubtle),
          )
        : null;

    if (hasTexture) {
      Widget content = SizedBox.fromSize(
        size: textureSize.value,
        child: Stack(
          clipBehavior: Clip.none,
          children: [
            LineHighlight(cursorInfo: displayCursor.value, isFocused: isFocused, enabled: lineHighlightEnabled),
            SizedBox.expand(child: Texture(textureId: textureId.value!)),
            Cursor(cursorInfo: displayCursor.value, isFocused: isFocused),
            ElementOverlay(pageIndex: pageIndex),
            if (layout.isPaginated && margins != null)
              Positioned.fill(
                child: IgnorePointer(
                  child: CustomPaint(
                    painter: _CropMarkerPainter(
                      marginTop: margins.pageMarginTop,
                      marginBottom: margins.pageMarginBottom,
                      marginLeft: margins.pageMarginLeft,
                      marginRight: margins.pageMarginRight,
                      color: context.colors.textDefault.withValues(alpha: 0.15),
                    ),
                  ),
                ),
              ),
          ],
        ),
      );

      if (pageDecoration != null) {
        content = DecoratedBox(decoration: pageDecoration, child: content);
      }

      return Padding(
        padding: EdgeInsets.only(bottom: bottomGap),
        child: content,
      );
    }

    return Container(
      height: pageHeight,
      margin: EdgeInsets.only(bottom: bottomGap),
      decoration: pageDecoration,
      child: const SizedBox.shrink(),
    );
  }
}

class _CropMarkerPainter extends CustomPainter {
  _CropMarkerPainter({
    required this.marginTop,
    required this.marginBottom,
    required this.marginLeft,
    required this.marginRight,
    required this.color,
  });

  final double marginTop;
  final double marginBottom;
  final double marginLeft;
  final double marginRight;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1;

    final pageWidth = size.width;
    final pageHeight = size.height;

    final path = Path()
      ..moveTo(marginLeft, marginTop - _cropMarkerSize)
      ..lineTo(marginLeft, marginTop)
      ..lineTo(marginLeft - _cropMarkerSize, marginTop)
      ..moveTo(pageWidth - marginRight, marginTop - _cropMarkerSize)
      ..lineTo(pageWidth - marginRight, marginTop)
      ..lineTo(pageWidth - marginRight + _cropMarkerSize, marginTop)
      ..moveTo(marginLeft, pageHeight - marginBottom + _cropMarkerSize)
      ..lineTo(marginLeft, pageHeight - marginBottom)
      ..lineTo(marginLeft - _cropMarkerSize, pageHeight - marginBottom)
      ..moveTo(pageWidth - marginRight, pageHeight - marginBottom + _cropMarkerSize)
      ..lineTo(pageWidth - marginRight, pageHeight - marginBottom)
      ..lineTo(pageWidth - marginRight + _cropMarkerSize, pageHeight - marginBottom);

    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(_CropMarkerPainter oldDelegate) {
    return marginTop != oldDelegate.marginTop ||
        marginBottom != oldDelegate.marginBottom ||
        marginLeft != oldDelegate.marginLeft ||
        marginRight != oldDelegate.marginRight ||
        color != oldDelegate.color;
  }
}
