import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:typie/widgets/responsive_container.dart';

void main() {
  testWidgets('side gutter drags scroll the primary scroll view', (tester) async {
    final controller = ScrollController();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: PrimaryScrollController(
            controller: controller,
            child: ResponsiveContainer(
              maxWidth: 600,
              child: SingleChildScrollView(
                controller: controller,
                physics: const AlwaysScrollableScrollPhysics(),
                child: const SizedBox(height: 2000),
              ),
            ),
          ),
        ),
      ),
    );

    expect(controller.offset, 0);

    final origin = tester.getTopLeft(find.byType(ResponsiveContainer));

    await tester.dragFrom(origin + const Offset(12, 300), const Offset(0, -240));
    await tester.pumpAndSettle();

    expect(controller.offset, greaterThan(0));
  });

  testWidgets('side gutter drags do not throw when primary controller has multiple positions', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ResponsiveContainer(
            maxWidth: 600,
            child: Column(
              children: [
                Expanded(
                  child: SingleChildScrollView(
                    primary: true,
                    physics: AlwaysScrollableScrollPhysics(),
                    child: SizedBox(height: 1600),
                  ),
                ),
                Expanded(
                  child: SingleChildScrollView(
                    primary: true,
                    physics: AlwaysScrollableScrollPhysics(),
                    child: SizedBox(height: 1600),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );

    final origin = tester.getTopLeft(find.byType(ResponsiveContainer));

    await tester.dragFrom(origin + const Offset(12, 300), const Offset(0, -240));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
  });

  testWidgets('side gutter drags scroll an explicit controller even when primary is unused', (tester) async {
    final controller = ScrollController();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ResponsiveContainer(
            maxWidth: 600,
            child: SingleChildScrollView(
              controller: controller,
              primary: false,
              physics: const AlwaysScrollableScrollPhysics(),
              child: const SizedBox(height: 2000),
            ),
          ),
        ),
      ),
    );

    expect(controller.offset, 0);

    final origin = tester.getTopLeft(find.byType(ResponsiveContainer));

    await tester.dragFrom(origin + const Offset(12, 300), const Offset(0, -240));
    await tester.pumpAndSettle();

    expect(controller.offset, greaterThan(0));
  });
}
