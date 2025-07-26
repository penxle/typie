import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/tappable.dart';

class FindReplaceBottomSheet extends HookWidget {
  const FindReplaceBottomSheet({required this.scope, super.key});

  final EditorStateScope scope;

  @override
  Widget build(BuildContext context) {
    final currentIndex = useState(0);
    final totalCount = useState(0);

    final findTextController = useTextEditingController();
    final replaceTextController = useTextEditingController();

    final findText = useListenableSelector(findTextController, () => findTextController.text);

    final findFocusNode = useFocusNode();
    final replaceFocusNode = useFocusNode();

    final webViewController = useValueListenable(scope.webViewController);

    final findHasFocus = useListenableSelector(findFocusNode, () => findFocusNode.hasFocus);
    final replaceHasFocus = useListenableSelector(replaceFocusNode, () => replaceFocusNode.hasFocus);

    useEffect(() {
      return () {
        unawaited(webViewController?.callProcedure('clearSearch'));
      };
    }, []);

    Future<void> findNext() async {
      final result = await webViewController?.callProcedure('findNext') as Map<String, dynamic>;
      currentIndex.value = result['currentIndex'] as int;
    }

    Future<void> findPrevious() async {
      final result = await webViewController?.callProcedure('findPrevious') as Map<String, dynamic>;
      currentIndex.value = result['currentIndex'] as int;
    }

    Future<void> replace() async {
      final result =
          await webViewController?.callProcedure('replace', replaceTextController.text) as Map<String, dynamic>;

      currentIndex.value = result['currentIndex'] as int;
      totalCount.value = result['totalCount'] as int;
    }

    Future<void> replaceAll() async {
      await webViewController?.callProcedure('replaceAll', replaceTextController.text);

      currentIndex.value = -1;
      totalCount.value = 0;
    }

    useAsyncEffect(() async {
      final result = await webViewController?.callProcedure('search', findText) as Map<String, dynamic>;

      currentIndex.value = result['currentIndex'] as int;
      totalCount.value = result['totalCount'] as int;

      return null;
    }, [findText]);

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
                              onSubmitted: (_) async {
                                findFocusNode.requestFocus();
                                await findNext();
                              },
                            ),
                          ),
                          if (findText.isNotEmpty)
                            Text(
                              totalCount.value > 0 ? '${currentIndex.value + 1} / ${totalCount.value}' : '결과 없음',
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
                          onSubmitted: (_) async {
                            replaceFocusNode.requestFocus();
                            await replace();
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
                        onTap: () async {
                          await findPrevious();
                        },
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
                        onTap: () async {
                          await findNext();
                        },
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
                        onTap: () async {
                          await replace();
                        },
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
                        onTap: () async {
                          await replaceAll();
                        },
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
