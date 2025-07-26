import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/tappable.dart';

class FindReplaceBottomSheet extends HookWidget {
  const FindReplaceBottomSheet({required this.scope, super.key});

  final EditorStateScope scope;

  @override
  Widget build(BuildContext context) {
    final findText = useState('');
    final replaceText = useState('');
    final currentMatch = useState(0);
    final totalMatches = useState(0);
    final findController = useTextEditingController();
    final replaceController = useTextEditingController();
    final findFocusNode = useFocusNode();
    final replaceFocusNode = useFocusNode();
    final webViewController = useValueListenable(scope.webViewController);

    final findHasFocus = useListenableSelector(findFocusNode, () => findFocusNode.hasFocus);
    final replaceHasFocus = useListenableSelector(replaceFocusNode, () => replaceFocusNode.hasFocus);

    useEffect(() {
      findController.addListener(() {
        findText.value = findController.text;
      });
      replaceController.addListener(() {
        replaceText.value = replaceController.text;
      });

      return () async {
        await webViewController?.callProcedure('clearSearchHighlights');
      };
    }, []);

    Future<void> updateMatches() async {
      if (findText.value.isEmpty) {
        totalMatches.value = 0;
        currentMatch.value = 0;
        await webViewController?.callProcedure('clearSearchHighlights');
        return;
      }

      final result =
          await webViewController?.callProcedure('search', {'text': findText.value}) as Map<String, dynamic>?;
      if (result != null) {
        totalMatches.value = result['totalMatches'] as int? ?? 0;
        currentMatch.value = result['currentMatch'] as int? ?? 0;
      }
    }

    Future<void> findNext() async {
      if (findText.value.isEmpty || totalMatches.value == 0) {
        return;
      }
      final result = await webViewController?.callProcedure('findNext') as Map<String, dynamic>?;
      if (result != null) {
        currentMatch.value = result['currentMatch'] as int? ?? 0;
      }
    }

    Future<void> findPrevious() async {
      if (findText.value.isEmpty || totalMatches.value == 0) {
        return;
      }
      final result = await webViewController?.callProcedure('findPrevious') as Map<String, dynamic>?;
      if (result != null) {
        currentMatch.value = result['currentMatch'] as int? ?? 0;
      }
    }

    Future<void> replace() async {
      if (findText.value.isEmpty || totalMatches.value == 0) {
        return;
      }
      final result =
          await webViewController?.callProcedure('replace', {'replaceText': replaceText.value})
              as Map<String, dynamic>?;
      if (result != null && result['success'] == true) {
        totalMatches.value = result['totalMatches'] as int? ?? 0;
        currentMatch.value = result['currentMatch'] as int? ?? 0;
      }
    }

    Future<void> replaceAll() async {
      if (findText.value.isEmpty) {
        return;
      }
      final result =
          await webViewController?.callProcedure('replaceAll', {
                'findText': findText.value,
                'replaceText': replaceText.value,
              })
              as Map<String, dynamic>?;
      if (result != null) {
        totalMatches.value = result['totalMatches'] as int? ?? 0;
        currentMatch.value = result['currentMatch'] as int? ?? 0;
      }
    }

    useEffect(() {
      unawaited(updateMatches());

      return null;
    }, [findText.value]);

    return AppBottomSheet(
      padding: const Pad(horizontal: 20, vertical: 16),
      includeBottomPadding: false,
      child: Padding(
        padding: EdgeInsets.only(bottom: MediaQuery.of(context).viewInsets.bottom),
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
                              controller: findController,
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
                              onSubmitted: (_) => findNext(),
                            ),
                          ),
                          if (findText.value.isNotEmpty)
                            Text(
                              totalMatches.value > 0 ? '${currentMatch.value + 1} / ${totalMatches.value}' : '결과 없음',
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
                          controller: replaceController,
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
                          onSubmitted: (_) => replace(),
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
                        onTap: () async {
                          if (findText.value.isNotEmpty) {
                            await findPrevious();
                          }
                        },
                        child: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: findText.value.isNotEmpty
                                  ? context.colors.borderStrong
                                  : context.colors.borderDefault,
                            ),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Center(
                            child: Icon(
                              LucideLightIcons.arrow_up,
                              size: 18,
                              color: findText.value.isNotEmpty ? context.colors.textDefault : context.colors.textFaint,
                            ),
                          ),
                        ),
                      ),
                      Tappable(
                        onTap: () async {
                          if (findText.value.isNotEmpty) {
                            await findNext();
                          }
                        },
                        child: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: findText.value.isNotEmpty
                                  ? context.colors.borderStrong
                                  : context.colors.borderDefault,
                            ),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Center(
                            child: Icon(
                              LucideLightIcons.arrow_down,
                              size: 18,
                              color: findText.value.isNotEmpty ? context.colors.textDefault : context.colors.textFaint,
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
                        onTap: () async {
                          if (findText.value.isNotEmpty) {
                            await replace();
                          }
                        },
                        child: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: findText.value.isNotEmpty
                                  ? context.colors.borderStrong
                                  : context.colors.borderDefault,
                            ),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Center(
                            child: Icon(
                              LucideLightIcons.replace,
                              size: 18,
                              color: findText.value.isNotEmpty ? context.colors.textDefault : context.colors.textFaint,
                            ),
                          ),
                        ),
                      ),
                      Tappable(
                        onTap: () async {
                          if (findText.value.isNotEmpty) {
                            await replaceAll();
                          }
                        },
                        child: Container(
                          width: 36,
                          height: 36,
                          decoration: BoxDecoration(
                            border: Border.all(
                              color: findText.value.isNotEmpty
                                  ? context.colors.borderStrong
                                  : context.colors.borderDefault,
                            ),
                            borderRadius: BorderRadius.circular(6),
                          ),
                          child: Center(
                            child: Icon(
                              LucideLightIcons.replace_all,
                              size: 18,
                              color: findText.value.isNotEmpty ? context.colors.textDefault : context.colors.textFaint,
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
