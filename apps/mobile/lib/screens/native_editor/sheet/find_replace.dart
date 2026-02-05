import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/state/state.dart';
import 'package:typie/widgets/tappable.dart';

class FindReplaceSheet extends HookWidget {
  const FindReplaceSheet({required this.controller, super.key});

  final EditorController controller;

  @override
  Widget build(BuildContext context) {
    final findTextController = useTextEditingController();
    final replaceTextController = useTextEditingController();

    final findText = useListenableSelector(findTextController, () => findTextController.text);

    final findFocusNode = useFocusNode();
    final replaceFocusNode = useFocusNode();

    final findHasFocus = useListenableSelector(findFocusNode, () => findFocusNode.hasFocus);
    final replaceHasFocus = useListenableSelector(replaceFocusNode, () => replaceFocusNode.hasFocus);

    final debounceTimer = useRef<Timer?>(null);

    final state = useListenable(controller);

    useEffect(() {
      return () {
        debounceTimer.value?.cancel();
        controller.dispatch({'type': 'clearSearch'});
      };
    }, []);

    useEffect(() {
      debounceTimer.value?.cancel();
      if (findText.isEmpty) {
        controller.dispatch({'type': 'clearSearch'});
        return null;
      }
      debounceTimer.value = Timer(const Duration(milliseconds: 150), () {
        controller.dispatch({'type': 'search', 'query': findText, 'matchWholeWord': false});
      });
      return null;
    }, [findText]);

    void findNext() {
      controller.dispatch({'type': 'findNext'});
    }

    void findPrevious() {
      controller.dispatch({'type': 'findPrevious'});
    }

    void replace() {
      controller.dispatch({'type': 'replace', 'replacement': replaceTextController.text});
    }

    void replaceAll() {
      controller.dispatch({'type': 'replaceAll', 'replacement': replaceTextController.text});
    }

    final totalCount = state.state.searchTotalCount;
    final currentIndex = state.state.searchCurrentIndex;

    final mediaQuery = MediaQuery.of(context);

    return AppBottomSheet(
      padding: const Pad(horizontal: 20, vertical: 16),
      includeBottomPadding: false,
      child: Padding(
        padding: Pad(bottom: mediaQuery.padding.bottom + mediaQuery.viewInsets.bottom),
        child: Row(
          spacing: 12,
          children: [
            Expanded(
              child: Column(
                spacing: 12,
                children: [
                  Container(
                    height: 44,
                    decoration: BoxDecoration(
                      border: Border.all(
                        color: findHasFocus ? context.colors.borderStrong : context.colors.borderDefault,
                      ),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Padding(
                      padding: const Pad(horizontal: 16),
                      child: Row(
                        children: [
                          Expanded(
                            child: TextField(
                              controller: findTextController,
                              focusNode: findFocusNode,
                              decoration: InputDecoration.collapsed(
                                hintText: '찾기',
                                hintStyle: TextStyle(
                                  fontSize: 16,
                                  fontWeight: FontWeight.w500,
                                  color: context.colors.textDisabled,
                                ),
                              ),
                              style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                              cursorColor: context.colors.textDefault,
                              autofocus: true,
                              textInputAction: TextInputAction.search,
                              onSubmitted: (_) {
                                findFocusNode.requestFocus();
                                findNext();
                              },
                            ),
                          ),
                          if (findText.isNotEmpty)
                            Text(
                              totalCount > 0 ? '${currentIndex + 1} / $totalCount' : '결과 없음',
                              style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
                            ),
                        ],
                      ),
                    ),
                  ),
                  Container(
                    height: 44,
                    decoration: BoxDecoration(
                      border: Border.all(
                        color: replaceHasFocus ? context.colors.borderStrong : context.colors.borderDefault,
                      ),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Align(
                      child: Padding(
                        padding: const Pad(horizontal: 16),
                        child: TextField(
                          controller: replaceTextController,
                          focusNode: replaceFocusNode,
                          decoration: InputDecoration.collapsed(
                            hintText: '바꾸기',
                            hintStyle: TextStyle(
                              fontSize: 16,
                              fontWeight: FontWeight.w500,
                              color: context.colors.textDisabled,
                            ),
                          ),
                          style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                          cursorColor: context.colors.textDefault,
                          textInputAction: TextInputAction.go,
                          onSubmitted: (_) {
                            replaceFocusNode.requestFocus();
                            replace();
                          },
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            Column(
              spacing: 12,
              children: [
                SizedBox(
                  height: 44,
                  child: Row(
                    spacing: 8,
                    children: [
                      Tappable(
                        onTap: findPrevious,
                        child: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: findText.isNotEmpty ? context.colors.borderStrong : context.colors.borderDefault,
                            ),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Center(
                            child: Icon(
                              LucideLightIcons.arrow_up,
                              size: 18,
                              color: findText.isNotEmpty ? context.colors.textDefault : context.colors.textFaint,
                            ),
                          ),
                        ),
                      ),
                      Tappable(
                        onTap: findNext,
                        child: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: findText.isNotEmpty ? context.colors.borderStrong : context.colors.borderDefault,
                            ),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Center(
                            child: Icon(
                              LucideLightIcons.arrow_down,
                              size: 18,
                              color: findText.isNotEmpty ? context.colors.textDefault : context.colors.textFaint,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                SizedBox(
                  height: 44,
                  child: Row(
                    spacing: 8,
                    children: [
                      Tappable(
                        onTap: replace,
                        child: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: findText.isNotEmpty ? context.colors.borderStrong : context.colors.borderDefault,
                            ),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Center(
                            child: Icon(
                              LucideLightIcons.replace,
                              size: 18,
                              color: findText.isNotEmpty ? context.colors.textDefault : context.colors.textFaint,
                            ),
                          ),
                        ),
                      ),
                      Tappable(
                        onTap: replaceAll,
                        child: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: findText.isNotEmpty ? context.colors.borderStrong : context.colors.borderDefault,
                            ),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Center(
                            child: Icon(
                              LucideLightIcons.replace_all,
                              size: 18,
                              color: findText.isNotEmpty ? context.colors.textDefault : context.colors.textFaint,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
