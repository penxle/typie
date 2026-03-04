import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:typie/screens/native_editor/view/zoom.dart';

class _NonGestureBouncingScrollPhysics extends BouncingScrollPhysics {
  const _NonGestureBouncingScrollPhysics({super.parent});

  @override
  _NonGestureBouncingScrollPhysics applyTo(ScrollPhysics? ancestor) {
    return _NonGestureBouncingScrollPhysics(parent: buildParent(ancestor));
  }

  @override
  bool shouldAcceptUserOffset(ScrollMetrics position) => false;
}

void main() {
  testWidgets('resolveScrollPosition returns the only attached position', (tester) async {
    final controller = ScrollController();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(controller: controller, child: const SizedBox(width: 100, height: 1200)),
        ),
      ),
    );

    final resolved = resolveScrollPosition(controller);
    expect(resolved, isNotNull);
    expect(controller.positions.length, 1);
    expect(identical(resolved, controller.positions.first), isTrue);
  });

  testWidgets('resolveScrollPosition prefers active out-of-range position when multiple are attached', (tester) async {
    final controller = ScrollController();

    Widget buildScroller() {
      return SingleChildScrollView(
        controller: controller,
        physics: const _NonGestureBouncingScrollPhysics(),
        child: const SizedBox(width: 100, height: 1200),
      );
    }

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Column(
            children: [
              Expanded(child: buildScroller()),
              Expanded(child: buildScroller()),
            ],
          ),
        ),
      ),
    );

    expect(controller.positions.length, 2);
    final positions = controller.positions.toList(growable: false);
    final first = positions[0];
    final second = positions[1];

    first.jumpTo(first.maxScrollExtent);
    second.jumpTo(second.maxScrollExtent);
    await tester.pump();

    final drag =
        second.drag(
          DragStartDetails(globalPosition: const Offset(150, 560), localPosition: const Offset(150, 560)),
          () {},
        )..update(
          DragUpdateDetails(
            globalPosition: const Offset(150, 500),
            localPosition: const Offset(150, 500),
            delta: const Offset(0, -60),
            primaryDelta: -60,
          ),
        );
    await tester.pump();

    expect(first.outOfRange, isFalse);
    expect(second.outOfRange, isTrue);

    final resolved = resolveScrollPosition(controller);
    expect(resolved, isNotNull);
    expect(identical(resolved, second), isTrue);

    drag.end(DragEndDetails(velocity: const Velocity(pixelsPerSecond: Offset(0, -200)), primaryVelocity: -200));
  });
}
