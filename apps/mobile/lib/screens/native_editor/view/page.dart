import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/native/editor_texture_renderer.dart';
import 'package:typie/screens/native_editor/external/overlay.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/screens/native_editor/view/cursor.dart';
import 'package:typie/screens/native_editor/view/line_highlight.dart';
import 'package:typie/screens/native_editor/view/scope.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';
import 'package:typie/services/preference.dart';

const _cropMarkerSize = 32.0;
const _renderRetryBaseDelayMs = 32;
const _renderRetryMaxDelayMs = 512;
const _renderRetryMaxAttempts = 6;

class PageItem extends HookWidget {
  const PageItem({required this.pageIndex, super.key});

  final int pageIndex;

  @override
  Widget build(BuildContext context) {
    final scope = ContentScope.of(context);
    final pref = useService<Pref>();
    final editorState = scope.controller.state;

    final layout = editorState.layout!;
    final cursor = editorState.cursor;
    final pageCursor = cursor?.pageIdx == pageIndex ? cursor : null;
    final isFocused = editorState.isFocused;
    final isPaginated = layout is PaginatedLayout;
    final margins = layout is PaginatedLayout ? layout : null;
    final page = editorState.pages.elementAtOrNull(pageIndex);
    final logicalPageWidth = margins?.pageWidth ?? page?.width ?? 0.0;
    final logicalPageHeight = margins?.pageHeight ?? page?.height ?? 0.0;
    final displayZoom = useValueListenable(scope.displayZoom);
    final renderZoom = useValueListenable(scope.renderZoom);
    final effectiveDisplayZoom = isPaginated ? displayZoom : 1.0;
    final effectiveRenderZoom = isPaginated ? renderZoom : 1.0;
    final bottomGap = scope.geometry.gapAfterPage(pageIndex);
    final pageHeight = scope.geometry.pageHeightAt(pageIndex);

    final editor = scope.editor;
    final renderVersion = editorState.renderVersion;
    final lineHighlightEnabled = pref.lineHighlightEnabled;

    final renderer = useRef<EditorTextureRenderer?>(null);
    final textureId = useState<int?>(null);
    final textureSize = useState<Size?>(null);
    final isMounted = useRef(true);
    final retryAttempts = useRef(0);
    final retryTimer = useRef<Timer?>(null);
    final isRenderTaskRunning = useRef(false);
    final hasQueuedRender = useRef(false);
    final latestLogicalSize = useRef<Size>(Size.zero)..value = Size(logicalPageWidth, logicalPageHeight);

    void resetRetryState() {
      retryAttempts.value = 0;
      retryTimer.value?.cancel();
      retryTimer.value = null;
    }

    void scheduleRetry(Future<void> Function() task) {
      if (!isMounted.value) {
        return;
      }
      if (retryAttempts.value >= _renderRetryMaxAttempts) {
        return;
      }

      retryTimer.value?.cancel();
      retryAttempts.value += 1;

      final exponent = retryAttempts.value - 1;
      final delayMs = (_renderRetryBaseDelayMs * (1 << exponent)).clamp(
        _renderRetryBaseDelayMs,
        _renderRetryMaxDelayMs,
      );

      retryTimer.value = Timer(Duration(milliseconds: delayMs), () {
        retryTimer.value = null;
        if (!isMounted.value) {
          return;
        }
        unawaited(task());
      });
    }

    Future<void> render() async {
      if (isRenderTaskRunning.value) {
        hasQueuedRender.value = true;
        return;
      }
      isRenderTaskRunning.value = true;

      renderer.value ??= EditorTextureRenderer(editor: editor);
      final r = renderer.value!;
      try {
        while (isMounted.value) {
          hasQueuedRender.value = false;

          if (r.textureId == null) {
            await r.create(pageIndex);
            if (!isMounted.value) {
              return;
            }
          }
          if (r.textureId == null) {
            scheduleRetry(render);
            return;
          }

          final didRender = await r.render(pageIndex);
          if (!isMounted.value) {
            return;
          }
          if (!didRender) {
            scheduleRetry(render);
            return;
          }

          resetRetryState();
          textureId.value = r.textureId;
          textureSize.value = latestLogicalSize.value;

          final pending = scope.pendingScroll.value;
          if (pending != null) {
            scope.pendingScroll.value = null;
            pending();
          }

          if (!hasQueuedRender.value) {
            return;
          }
        }
      } finally {
        isRenderTaskRunning.value = false;
      }
    }

    useEffect(() {
      final timer = Timer(const Duration(milliseconds: 150), () {
        unawaited(render());
      });
      return () {
        timer.cancel();
        retryTimer.value?.cancel();
        retryTimer.value = null;
        isMounted.value = false;
        unawaited(renderer.value?.dispose());
      };
    }, const []);

    useEffect(() {
      resetRetryState();
      if (renderer.value?.textureId != null) {
        unawaited(render());
      }
      return null;
    }, [renderVersion, effectiveRenderZoom]);

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
      final baseSize = textureSize.value!;
      final backgroundOverlayLayer = SizedBox.fromSize(
        size: baseSize,
        child: Stack(
          clipBehavior: Clip.none,
          children: [LineHighlight(cursorInfo: pageCursor, isFocused: isFocused, enabled: lineHighlightEnabled)],
        ),
      );
      final foregroundOverlayLayer = SizedBox.fromSize(
        size: baseSize,
        child: Stack(
          clipBehavior: Clip.none,
          children: [
            _SearchHighlightOverlay(pageIndex: pageIndex, overlays: editorState.search.overlays),
            _SpellcheckOverlay(pageIndex: pageIndex, overlays: editorState.spellcheck.overlays),
            _AiFeedbackOverlay(pageIndex: pageIndex, overlays: editorState.aiFeedback.overlays),
            _RemarkHighlightOverlay(pageIndex: pageIndex, controller: scope.controller),
            Cursor(cursorInfo: pageCursor, isFocused: isFocused),
            ElementOverlay(pageIndex: pageIndex),
            if (isPaginated && margins != null)
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

      Widget content;
      if (isPaginated && !isUnitZoom(effectiveDisplayZoom)) {
        final scaledSize = Size(baseSize.width * effectiveDisplayZoom, baseSize.height * effectiveDisplayZoom);
        final scaledBackgroundOverlay = OverflowBox(
          alignment: Alignment.topLeft,
          minWidth: baseSize.width,
          maxWidth: baseSize.width,
          minHeight: baseSize.height,
          maxHeight: baseSize.height,
          child: Transform.scale(
            alignment: Alignment.topLeft,
            scale: effectiveDisplayZoom,
            child: backgroundOverlayLayer,
          ),
        );
        final scaledForegroundOverlay = OverflowBox(
          alignment: Alignment.topLeft,
          minWidth: baseSize.width,
          maxWidth: baseSize.width,
          minHeight: baseSize.height,
          maxHeight: baseSize.height,
          child: Transform.scale(
            alignment: Alignment.topLeft,
            scale: effectiveDisplayZoom,
            child: foregroundOverlayLayer,
          ),
        );
        content = SizedBox.fromSize(
          size: scaledSize,
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              Positioned.fill(child: scaledBackgroundOverlay),
              Positioned.fill(child: Texture(textureId: textureId.value!)),
              Positioned.fill(child: scaledForegroundOverlay),
            ],
          ),
        );
      } else {
        content = SizedBox.fromSize(
          size: baseSize,
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              backgroundOverlayLayer,
              SizedBox.expand(child: Texture(textureId: textureId.value!)),
              foregroundOverlayLayer,
            ],
          ),
        );
      }

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

class _SpellcheckOverlay extends StatelessWidget {
  const _SpellcheckOverlay({required this.pageIndex, required this.overlays});

  final int pageIndex;
  final List<SpellcheckOverlayInfo> overlays;

  static const _wavyColor = Color(0xFFDC2626);

  @override
  Widget build(BuildContext context) {
    final pageOverlays = overlays.where((o) => o.pageIdx == pageIndex);
    if (pageOverlays.isEmpty) {
      return const SizedBox.shrink();
    }

    return IgnorePointer(
      child: Stack(
        children: [
          for (final overlay in pageOverlays)
            for (final bound in overlay.bounds)
              Positioned(
                left: bound.x,
                top: bound.y + bound.ascent + 2,
                width: bound.width,
                height: 4,
                child: CustomPaint(painter: _WavyUnderlinePainter(color: _wavyColor)),
              ),
        ],
      ),
    );
  }
}

class _WavyUnderlinePainter extends CustomPainter {
  _WavyUnderlinePainter({required this.color});

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1;

    const waveLength = 6.0;
    const amplitude = 1.5;

    final path = Path()..moveTo(0, amplitude);

    var x = 0.0;
    var i = 0;
    while (x < size.width) {
      final nextX = (x + waveLength / 2).clamp(0.0, size.width);
      final controlY = i.isEven ? 0.0 : amplitude * 2;
      final endY = i.isEven ? amplitude * 2 : 0.0;
      path.quadraticBezierTo((x + nextX) / 2, controlY, nextX, endY);
      x = nextX;
      i++;
    }

    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(_WavyUnderlinePainter oldDelegate) => color != oldDelegate.color;
}

class _SearchHighlightOverlay extends StatelessWidget {
  const _SearchHighlightOverlay({required this.pageIndex, required this.overlays});

  final int pageIndex;
  final List<SearchOverlayInfo> overlays;

  static const _currentColor = Color.fromRGBO(255, 165, 0, 0.5);
  static const _matchColor = Color.fromRGBO(255, 255, 0, 0.5);

  @override
  Widget build(BuildContext context) {
    final pageOverlays = overlays.where((o) => o.pageIdx == pageIndex);
    if (pageOverlays.isEmpty) {
      return const SizedBox.shrink();
    }

    return IgnorePointer(
      child: Stack(
        children: [
          for (final overlay in pageOverlays)
            for (final bound in overlay.bounds)
              Positioned(
                left: bound.x,
                top: bound.y,
                width: bound.width,
                height: bound.height,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: overlay.isCurrent ? _currentColor : _matchColor,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
        ],
      ),
    );
  }
}

class _AiFeedbackOverlay extends StatelessWidget {
  const _AiFeedbackOverlay({required this.pageIndex, required this.overlays});

  final int pageIndex;
  final List<AiFeedbackOverlayInfo> overlays;

  @override
  Widget build(BuildContext context) {
    final pageOverlays = overlays.where((o) => o.pageIdx == pageIndex && o.isActive);
    if (pageOverlays.isEmpty) {
      return const SizedBox.shrink();
    }

    return IgnorePointer(
      child: Stack(
        children: [
          for (final overlay in pageOverlays)
            for (final bound in overlay.bounds)
              Positioned(
                left: bound.x,
                top: bound.y,
                width: bound.width,
                height: bound.height,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: context.colors.accentBrand.withValues(alpha: 0.15),
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
        ],
      ),
    );
  }
}

class _RemarkHighlightOverlay extends StatefulWidget {
  const _RemarkHighlightOverlay({required this.pageIndex, required this.controller});

  final int pageIndex;
  final EditorController controller;

  @override
  State<_RemarkHighlightOverlay> createState() => _RemarkHighlightOverlayState();
}

class _RemarkHighlightOverlayState extends State<_RemarkHighlightOverlay> with SingleTickerProviderStateMixin {
  late final AnimationController _animation;
  RemarkOverlayInfo? _target;

  @override
  void initState() {
    super.initState();
    _animation = AnimationController(vsync: this, duration: const Duration(milliseconds: 1500))
      ..addStatusListener((status) {
        if (status == AnimationStatus.completed) {
          setState(() => _target = null);
        }
      });
    widget.controller.remarkHighlightTarget.addListener(_onHighlight);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      _onHighlight();
    });
  }

  @override
  void dispose() {
    widget.controller.remarkHighlightTarget.removeListener(_onHighlight);
    _animation.dispose();
    super.dispose();
  }

  void _onHighlight() {
    final target = widget.controller.remarkHighlightTarget.value;
    if (target != null && target.pageIdx == widget.pageIndex) {
      setState(() => _target = target);
      unawaited(_animation.forward(from: 0));
    }
  }

  @override
  Widget build(BuildContext context) {
    final target = _target;
    if (target == null) {
      return const SizedBox.shrink();
    }

    return IgnorePointer(
      child: AnimatedBuilder(
        animation: _animation,
        builder: (context, _) {
          final t = _animation.value;
          // fast fade-in (0~0.15), hold (0.15~0.4), fade-out (0.4~1.0)
          final double opacity;
          if (t < 0.15) {
            opacity = t / 0.15;
          } else if (t < 0.4) {
            opacity = 1.0;
          } else {
            opacity = 1.0 - (t - 0.4) / 0.6;
          }

          return Stack(
            children: [
              Positioned(
                left: target.boundsX - 4,
                top: target.boundsY - 4,
                width: target.boundsWidth + 8,
                height: target.boundsHeight + 8,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: context.colors.accentBrand.withValues(alpha: 0.12 * opacity),
                    borderRadius: BorderRadius.circular(4),
                  ),
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}
