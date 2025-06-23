import 'dart:async';

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
    final bottomToolbarMode = useValueNotifier<BottomToolbarMode>(BottomToolbarMode.hidden);
    final secondaryToolbarMode = useValueNotifier<SecondaryToolbarMode>(SecondaryToolbarMode.hidden);

    final pageController = usePageController();
    final shouldInitialize = useState(false);

    return EditorStateScope(
      data: data,
      webViewController: webViewController,
      proseMirrorState: proseMirrorState,
      characterCountState: characterCountState,
      yjsState: yjsState,
      keyboardHeight: keyboardHeight,
      isKeyboardVisible: isKeyboardVisible,
      keyboardType: keyboardType,
      bottomToolbarMode: bottomToolbarMode,
      secondaryToolbarMode: secondaryToolbarMode,
      child: Stack(
        children: [
          PageView(
            controller: pageController,
            physics: const NeverScrollableScrollPhysics(),
            children: [
              Editor(slug: slug),
              Note(shouldInitialize: shouldInitialize.value),
            ],
            onPageChanged: (value) {
              shouldInitialize.value = value == 1;
            },
          ),
          Positioned(
            top: 0,
            left: 0,
            right: 0,
            height: MediaQuery.paddingOf(context).top + 52,
            child: RawGestureDetector(
              gestures: {
                HorizontalDragGestureRecognizer: GestureRecognizerFactoryWithHandlers<HorizontalDragGestureRecognizer>(
                  HorizontalDragGestureRecognizer.new,
                  (instance) {
                    Offset? dragStart;

                    instance
                      ..onStart = (details) {
                        dragStart = details.globalPosition;
                      }
                      ..onUpdate = (details) {
                        if (dragStart == null) {
                          return;
                        }

                        final delta = details.globalPosition.dx - dragStart!.dx;
                        unawaited(pageController.position.moveTo(pageController.position.pixels - delta));

                        dragStart = details.globalPosition;
                      }
                      ..onEnd = (details) {
                        final velocity = details.velocity.pixelsPerSecond.dx;

                        final page = pageController.page!;
                        final nextPage = velocity < 0 ? page.ceil() : page.floor();

                        unawaited(
                          pageController.animateToPage(
                            nextPage.clamp(0, 1),
                            duration: const Duration(milliseconds: 300),
                            curve: Curves.easeOut,
                          ),
                        );
                      };
                  },
                ),
              },
              behavior: HitTestBehavior.translucent,
              child: const SizedBox.expand(),
            ),
          ),
        ],
      ),
    );
  }
}
