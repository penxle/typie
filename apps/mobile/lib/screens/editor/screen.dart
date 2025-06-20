import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/__generated__/editor_query.data.gql.dart';
import 'package:typie/screens/editor/editor.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
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

    final textEditingController = useTextEditingController();
    final pageController = usePageController();

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
            onPageChanged: (value) {
              if (value == 1) {
                if (yjsState.value?.note != null && textEditingController.text.isEmpty) {
                  textEditingController.text = yjsState.value!.note;
                }
              }
            },
            children: [
              Editor(slug: slug),
              _PanelNote(controller: textEditingController),
            ],
          ),
          Positioned(
            top: 0,
            left: 0,
            right: 0,
            height: kToolbarHeight + 52 + 16, // toolbar + heading + padding
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

class _PanelNote extends StatelessWidget {
  const _PanelNote({required this.controller});

  final TextEditingController controller;

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);

    return Screen(
      padding: const Pad(all: 20),
      heading: const Heading(
        titleIcon: LucideLightIcons.notebook_tabs,
        title: '작성 노트',
        backgroundColor: AppColors.white,
      ),
      backgroundColor: AppColors.white,
      child: TextField(
        controller: controller,
        smartDashesType: SmartDashesType.disabled,
        smartQuotesType: SmartQuotesType.disabled,
        autocorrect: false,
        keyboardType: TextInputType.multiline,
        maxLines: null,
        expands: true,
        textAlignVertical: TextAlignVertical.top,
        decoration: const InputDecoration(
          hintText: '포스트에 대해 기억할 내용이나 작성에 도움이 되는 내용이 있다면 자유롭게 적어보세요',
          hintStyle: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: AppColors.gray_300),
        ),
        onChanged: (value) async {
          await scope.command('note', attrs: {'note': value});
        },
      ),
    );
  }
}
