import 'dart:math';

import 'package:auto_route/auto_route.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/screens/editor/__generated__/editor_query.data.gql.dart';
import 'package:typie/screens/editor/editor.dart';
import 'package:typie/screens/editor/note.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/webview.dart';

@RoutePage()
class EditorScreen extends HookWidget {
  const EditorScreen({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    final data = useValueNotifier<GEditorScreen_QueryData?>(null);
    final webViewController = useValueNotifier<WebViewController?>(null);
    final proseMirrorState = useValueNotifier<ProseMirrorState?>(null);
    final characterCountState = useValueNotifier<CharacterCountState?>(null);
    final yjsState = useValueNotifier<YJSState?>(null);
    final keyboardHeight = useValueNotifier<double>(0);
    final isKeyboardVisible = useValueNotifier<bool>(false);
    final keyboardType = useValueNotifier<KeyboardType>(KeyboardType.software);
    final mode = useValueNotifier<EditorMode>(EditorMode.editor);
    final bottomToolbarMode = useValueNotifier<BottomToolbarMode>(BottomToolbarMode.hidden);
    final secondaryToolbarMode = useValueNotifier<SecondaryToolbarMode>(SecondaryToolbarMode.hidden);

    final pageController = usePageController();
    final drag = useRef<Drag?>(null);

    return EditorStateScope(
      data: data,
      webViewController: webViewController,
      proseMirrorState: proseMirrorState,
      characterCountState: characterCountState,
      yjsState: yjsState,
      keyboardHeight: keyboardHeight,
      isKeyboardVisible: isKeyboardVisible,
      keyboardType: keyboardType,
      mode: mode,
      bottomToolbarMode: bottomToolbarMode,
      secondaryToolbarMode: secondaryToolbarMode,
      child: Material(
        color: AppColors.white,
        child: Stack(
          children: [
            PageView(
              controller: pageController,
              physics: const NeverScrollableScrollPhysics(),
              onPageChanged: (value) {
                mode.value = switch (value) {
                  0 => EditorMode.editor,
                  1 => EditorMode.note,
                  _ => throw UnimplementedError(),
                };
              },
              children: [
                Editor(slug: slug),
                Note(
                  onBack: () async {
                    drag.value?.cancel();
                    drag.value = null;

                    final details = DragStartDetails(localPosition: Offset.zero);
                    drag.value = pageController.position.drag(details, () {});

                    const duration = Duration(milliseconds: 150);
                    const steps = 30;
                    final stepDuration = duration.inMicroseconds ~/ steps;
                    final screenWidth = MediaQuery.of(context).size.width;

                    for (var i = 0; i < steps; i++) {
                      final progress = (i + 1) / steps;
                      final easeOutProgress = 1 - pow(1 - progress, 3);
                      final dx = screenWidth * easeOutProgress;

                      drag.value?.update(
                        DragUpdateDetails(
                          globalPosition: Offset(dx, 0),
                          localPosition: Offset(dx, 0),
                          delta: Offset(dx / steps, 0),
                          primaryDelta: dx / steps,
                        ),
                      );

                      await Future<void>.delayed(Duration(microseconds: stepDuration));
                    }

                    drag.value?.end(
                      DragEndDetails(velocity: const Velocity(pixelsPerSecond: Offset(600, 0)), primaryVelocity: 600),
                    );
                  },
                ),
              ],
            ),
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              height: MediaQuery.paddingOf(context).top + 52,
              child: GestureDetector(
                onHorizontalDragDown: (details) {
                  drag.value?.cancel();
                  drag.value = null;
                },
                onHorizontalDragStart: (details) {
                  drag.value = pageController.position.drag(
                    DragStartDetails(globalPosition: details.globalPosition, localPosition: details.localPosition),
                    () {},
                  );
                },
                onHorizontalDragUpdate: (details) {
                  drag.value?.update(
                    DragUpdateDetails(
                      globalPosition: details.globalPosition,
                      localPosition: details.localPosition,
                      delta: Offset(details.delta.dx, 0),
                      primaryDelta: details.delta.dx,
                    ),
                  );
                },
                onHorizontalDragEnd: (details) {
                  drag.value?.end(
                    DragEndDetails(
                      velocity: Velocity(pixelsPerSecond: Offset(details.velocity.pixelsPerSecond.dx, 0)),
                      primaryVelocity: details.velocity.pixelsPerSecond.dx,
                    ),
                  );
                  drag.value = null;
                },
                onHorizontalDragCancel: () {
                  drag.value?.cancel();
                  drag.value = null;
                },
                behavior: HitTestBehavior.translucent,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
