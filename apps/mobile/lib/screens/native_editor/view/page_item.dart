import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/native/editor_texture_renderer.dart';
import 'package:typie/screens/native_editor/cursor.dart';
import 'package:typie/screens/native_editor/external/overlay.dart';

const _cropMarkerSize = 32.0;

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
    required this.isPaginated,
    this.pageMarginTop = 0,
    this.pageMarginBottom = 0,
    this.pageMarginLeft = 0,
    this.pageMarginRight = 0,
    this.onRenderComplete,
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
  final bool isPaginated;
  final double pageMarginTop;
  final double pageMarginBottom;
  final double pageMarginLeft;
  final double pageMarginRight;
  final VoidCallback? onRenderComplete;

  @override
  Widget build(BuildContext context) {
    final renderer = useRef<EditorTextureRenderer?>(null);
    final textureId = useState<int?>(null);
    final textureSize = useState<Size?>(null);
    final isMounted = useRef(true);
    final displayCursor = useState<CursorInfo?>(cursorInfo);
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
      displayCursor.value = cursorInfo;
      onRenderComplete?.call();
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
        displayCursor.value = cursorInfo;
      }
      return null;
    }, [cursorInfo]);

    final hasTexture = textureId.value != null && textureSize.value != null;

    final pageDecoration = isPaginated
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
            EditorCursor(cursorInfo: displayCursor.value, isFocused: isFocused),
            ExternalElementOverlay(pageIndex: pageIndex),
            if (isPaginated)
              Positioned.fill(
                child: IgnorePointer(
                  child: CustomPaint(
                    painter: _CropMarkerPainter(
                      marginTop: pageMarginTop,
                      marginBottom: pageMarginBottom,
                      marginLeft: pageMarginLeft,
                      marginRight: pageMarginRight,
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
      child: const Center(child: CircularProgressIndicator()),
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
