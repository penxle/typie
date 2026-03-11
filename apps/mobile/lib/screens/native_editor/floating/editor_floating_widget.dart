import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/native_editor/floating/native_editor_floating_fade.dart';
import 'package:typie/widgets/overlay_heading.dart';

const _editorFloatingTopInset = OverlayHeading.height;

class NativeEditorFloatingWidget extends HookWidget {
  NativeEditorFloatingWidget({
    required this.child,
    required this.containerKey,
    required this.headerKey,
    required this.onPositionChanged,
    this.initialRelativePosition,
    this.isExpanded = false,
    this.onTap,
    super.key,
  }) : assert(
         initialRelativePosition == null ||
             (initialRelativePosition.dx >= 0.0 &&
                 initialRelativePosition.dx <= 1.0 &&
                 initialRelativePosition.dy >= 0.0 &&
                 initialRelativePosition.dy <= 1.0),
         'initialRelativePosition must be within 0.0–1.0',
       );

  final Widget child;
  final GlobalKey containerKey;
  final GlobalKey headerKey;
  final void Function(Offset relativePosition) onPositionChanged;
  final Offset? initialRelativePosition;
  final bool isExpanded;
  final void Function(bool isFaded)? onTap;

  Offset _clampPosition(Offset position, Size widgetSize, Rect movableBounds) {
    final maxX = movableBounds.width <= widgetSize.width ? movableBounds.left : movableBounds.right - widgetSize.width;
    final maxY = movableBounds.height <= widgetSize.height
        ? movableBounds.top
        : movableBounds.bottom - widgetSize.height;

    return Offset(position.dx.clamp(movableBounds.left, maxX), position.dy.clamp(movableBounds.top, maxY));
  }

  Rect _resolveMovableBounds({required BuildContext context, required RenderBox editorContainer}) {
    final containerSize = editorContainer.size;
    final mediaQuery = MediaQuery.of(context);
    final keyboardHeight = mediaQuery.viewInsets.bottom;
    final keyboardTop = mediaQuery.size.height - keyboardHeight;
    final containerTop = editorContainer.localToGlobal(Offset.zero).dy;
    final headingRenderBox = headerKey.currentContext?.findRenderObject() as RenderBox?;
    final visibleHeight = keyboardHeight <= 0
        ? containerSize.height
        : (keyboardTop - containerTop).clamp(0.0, containerSize.height);
    final headerBottom = headingRenderBox == null || !headingRenderBox.hasSize
        ? mediaQuery.padding.top + _editorFloatingTopInset
        : headingRenderBox.localToGlobal(Offset(0, headingRenderBox.size.height)).dy;
    final top = (headerBottom - containerTop).clamp(0.0, visibleHeight);

    return Rect.fromLTWH(0, top, containerSize.width, (visibleHeight - top).clamp(0.0, containerSize.height));
  }

  Offset _toAbsolutePosition(Offset relativePos, Rect movableBounds) {
    return Offset(
      movableBounds.left + relativePos.dx * movableBounds.width,
      movableBounds.top + relativePos.dy * movableBounds.height,
    );
  }

  Offset _toRelativePosition(Offset absolutePos, Rect movableBounds) {
    final w = movableBounds.width;
    final h = movableBounds.height;
    if (w == 0 || h == 0) {
      return Offset.zero;
    }
    return Offset((absolutePos.dx - movableBounds.left) / w, (absolutePos.dy - movableBounds.top) / h);
  }

  @override
  Widget build(BuildContext context) {
    final widgetKey = useMemoized(GlobalKey.new);
    final widgetSize = useState<Size?>(null);

    final mediaQuery = MediaQuery.of(context);
    final screenSize = mediaQuery.size;
    final relativePos = initialRelativePosition ?? const Offset(0.5, 0.5);
    final initialTop = (_editorFloatingTopInset + mediaQuery.padding.top).clamp(0.0, screenSize.height);
    final initialX = relativePos.dx * screenSize.width;
    final initialY = initialTop + relativePos.dy * (screenSize.height - initialTop);

    final position = useState<Offset>(Offset(initialX, initialY));
    final isDragging = useState(false);
    final currentRelativePosition = useState<Offset?>(initialRelativePosition);
    final previousEditorBounds = useState<Rect?>(null);

    final originalPosition = useState<Offset?>(null);
    final hasDragged = useState(false);
    final wasExpanded = useState(false);

    final fadeController = useNativeEditorFloatingFade();

    // NOTE: 확장 상태 변화 감지
    useEffect(() {
      if (isExpanded && !wasExpanded.value) {
        originalPosition.value = position.value;
        hasDragged.value = false;
        wasExpanded.value = true;

        WidgetsBinding.instance.addPostFrameCallback((_) {
          final renderBox = widgetKey.currentContext?.findRenderObject() as RenderBox?;
          final editorContainer = containerKey.currentContext?.findRenderObject() as RenderBox?;

          if (renderBox != null && editorContainer != null && renderBox.hasSize && editorContainer.hasSize) {
            final movableBounds = _resolveMovableBounds(context: context, editorContainer: editorContainer);
            final adjustedPosition = _clampPosition(position.value, renderBox.size, movableBounds);

            if (adjustedPosition != position.value) {
              position.value = adjustedPosition;
            }
          }
        });
      } else if (!isExpanded && wasExpanded.value) {
        if (!hasDragged.value && originalPosition.value != null) {
          position.value = originalPosition.value!;
        }
        originalPosition.value = null;
        hasDragged.value = false;
        wasExpanded.value = false;
      }
      return null;
    }, [isExpanded]);

    // NOTE: 초기 상대 위치에서 절대 위치 계산 (위젯 크기 측정 후)
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        final renderBox = widgetKey.currentContext?.findRenderObject() as RenderBox?;
        final editorContainer = containerKey.currentContext?.findRenderObject() as RenderBox?;

        if (renderBox != null && renderBox.hasSize && editorContainer != null && editorContainer.hasSize) {
          if (initialRelativePosition == null) {
            return;
          }

          final relativePos = initialRelativePosition!;
          currentRelativePosition.value = relativePos;

          final movableBounds = _resolveMovableBounds(context: context, editorContainer: editorContainer);
          final absolutePos = _toAbsolutePosition(relativePos, movableBounds);
          position.value = _clampPosition(absolutePos, renderBox.size, movableBounds);

          previousEditorBounds.value = movableBounds;
          widgetSize.value = renderBox.size;
        }
      });
      return null;
    }, []);

    // NOTE: 화면 크기 변경 시 위치 재계산
    useEffect(() {
      void updateWidgetSizeAndPosition() {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          final renderBox = widgetKey.currentContext?.findRenderObject() as RenderBox?;
          if (renderBox != null && renderBox.hasSize && widgetSize.value != renderBox.size) {
            widgetSize.value = renderBox.size;
          }

          // NOTE: 에디터 컨테이너 크기 변경 감지 및 위치 재계산
          if (!isDragging.value && previousEditorBounds.value != null) {
            final editorContainer = containerKey.currentContext?.findRenderObject() as RenderBox?;
            if (editorContainer != null && editorContainer.hasSize) {
              final currentEditorBounds = _resolveMovableBounds(context: context, editorContainer: editorContainer);

              // NOTE: 크기가 변경되었을 때만 위치 재계산
              if (previousEditorBounds.value != currentEditorBounds) {
                final relativePos = currentRelativePosition.value ?? initialRelativePosition;
                if (relativePos == null) {
                  return;
                }

                if (widgetSize.value != null) {
                  final absolutePos = _toAbsolutePosition(relativePos, currentEditorBounds);
                  final newPosition = _clampPosition(absolutePos, widgetSize.value!, currentEditorBounds);

                  if (position.value != newPosition) {
                    position.value = newPosition;
                  }
                }

                previousEditorBounds.value = currentEditorBounds;
              }
            }
          }
        });
      }

      updateWidgetSizeAndPosition();
      return null;
    });

    return AnimatedPositioned(
      duration: isDragging.value ? Duration.zero : const Duration(milliseconds: 250),
      curve: Curves.easeOutCubic,
      left: position.value.dx,
      top: position.value.dy,
      child: FadeTransition(
        opacity: fadeController.opacity,
        child: GestureDetector(
          onTap: () {
            final isFaded = fadeController.opacity.value < 1.0;
            fadeController.showImmediately();
            onTap?.call(isFaded);
          },
          onPanStart: (_) {
            isDragging.value = true;
            fadeController.showImmediately();
          },
          onPanUpdate: (details) {
            hasDragged.value = true;

            final editorContainer = containerKey.currentContext?.findRenderObject() as RenderBox?;

            if (widgetSize.value == null || editorContainer == null || !editorContainer.hasSize) {
              return;
            }

            final movableBounds = _resolveMovableBounds(context: context, editorContainer: editorContainer);

            final newPosition = Offset(position.value.dx + details.delta.dx, position.value.dy + details.delta.dy);

            final adjustedPosition = _clampPosition(newPosition, widgetSize.value!, movableBounds);

            position.value = adjustedPosition;
          },
          onPanEnd: (_) {
            isDragging.value = false;
            final editorContainer = containerKey.currentContext?.findRenderObject() as RenderBox?;
            if (editorContainer != null && editorContainer.hasSize) {
              final movableBounds = _resolveMovableBounds(context: context, editorContainer: editorContainer);
              final relativePos = _toRelativePosition(position.value, movableBounds);
              currentRelativePosition.value = relativePos;
              onPositionChanged(relativePos);
            }
          },
          onPanCancel: () {
            isDragging.value = false;
            final editorContainer = containerKey.currentContext?.findRenderObject() as RenderBox?;
            if (editorContainer != null && editorContainer.hasSize) {
              final movableBounds = _resolveMovableBounds(context: context, editorContainer: editorContainer);
              final relativePos = _toRelativePosition(position.value, movableBounds);
              currentRelativePosition.value = relativePos;
              onPositionChanged(relativePos);
            }
          },
          child: KeyedSubtree(key: widgetKey, child: child),
        ),
      ),
    );
  }
}
