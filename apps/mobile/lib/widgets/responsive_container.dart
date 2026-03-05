import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';

class ResponsiveContainer extends StatelessWidget {
  const ResponsiveContainer({required this.child, super.key, this.maxWidth, this.alignment = Alignment.topCenter});
  static const _responsiveBreakpoint = 600.0;

  final Widget child;
  final double? maxWidth;
  final Alignment alignment;

  @override
  Widget build(BuildContext context) {
    final screenWidth = MediaQuery.sizeOf(context).width;
    final effectiveMaxWidth = maxWidth ?? _responsiveBreakpoint;

    if (screenWidth < _responsiveBreakpoint) {
      return child;
    }

    final gutterWidth = (screenWidth - effectiveMaxWidth) / 2;

    return Stack(
      children: [
        Align(
          alignment: alignment,
          child: ConstrainedBox(
            constraints: BoxConstraints(maxWidth: effectiveMaxWidth),
            child: child,
          ),
        ),
        if (gutterWidth > 0)
          Positioned.fill(
            child: Row(
              children: [
                _ScrollForwardingGutter(width: gutterWidth, scopeContext: context),
                const Spacer(),
                _ScrollForwardingGutter(width: gutterWidth, scopeContext: context),
              ],
            ),
          ),
      ],
    );
  }
}

class _ScrollForwardingGutter extends HookWidget {
  const _ScrollForwardingGutter({required this.width, required this.scopeContext});

  final double width;
  final BuildContext scopeContext;

  @override
  Widget build(BuildContext context) {
    final drag = useRef<Drag?>(null);

    ScrollPosition? resolveFromDescendantScrollables() {
      final rootElement = scopeContext as Element;
      final candidates = <ScrollPosition>[];

      void visit(Element element) {
        if (element is StatefulElement && element.state is ScrollableState) {
          final state = element.state as ScrollableState;
          final position = state.position;
          if (position.axis == Axis.vertical) {
            candidates.add(position);
          }
        }
        element.visitChildren(visit);
      }

      rootElement.visitChildren(visit);
      if (candidates.isEmpty) {
        return null;
      }

      ScrollPosition? active;
      var maxViewportDimension = -1.0;
      for (final position in candidates) {
        if (position.viewportDimension > maxViewportDimension) {
          maxViewportDimension = position.viewportDimension;
          active = position;
        }
      }

      return active;
    }

    ScrollPosition? resolveFromPrimaryScrollController() {
      final controller = PrimaryScrollController.maybeOf(context);
      if (controller == null || !controller.hasClients) {
        return null;
      }

      ScrollPosition? active;
      var maxViewportDimension = -1.0;
      for (final position in controller.positions) {
        if (position.axis != Axis.vertical) {
          continue;
        }
        if (position.viewportDimension > maxViewportDimension) {
          maxViewportDimension = position.viewportDimension;
          active = position;
        }
      }

      return active;
    }

    ScrollPosition? resolveScrollPosition() {
      return resolveFromDescendantScrollables() ?? resolveFromPrimaryScrollController();
    }

    void disposeDrag() {
      drag.value = null;
    }

    void handleDragStart(DragStartDetails details) {
      drag.value?.cancel();

      final position = resolveScrollPosition();
      if (position == null) {
        disposeDrag();
        return;
      }

      drag.value = position.drag(details, disposeDrag);
    }

    void handleDragUpdate(DragUpdateDetails details) {
      drag.value?.update(details);
    }

    void handleDragEnd(DragEndDetails details) {
      drag.value?.end(details);
      disposeDrag();
    }

    void handleDragCancel() {
      drag.value?.cancel();
      disposeDrag();
    }

    useEffect(() {
      return () {
        drag.value?.cancel();
      };
    }, const []);

    return SizedBox(
      width: width,
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onVerticalDragStart: handleDragStart,
        onVerticalDragUpdate: handleDragUpdate,
        onVerticalDragEnd: handleDragEnd,
        onVerticalDragCancel: handleDragCancel,
      ),
    );
  }
}
