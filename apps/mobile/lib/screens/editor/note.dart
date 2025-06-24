import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

class Note extends HookWidget {
  const Note({super.key});

  @override
  Widget build(BuildContext context) {
    useAutomaticKeepAlive();

    final scope = EditorStateScope.of(context);
    final controller = useTextEditingController();
    final yjsState = useValueListenable(scope.yjsState);

    useEffect(() {
      controller.text = yjsState?.note ?? '';
      return null;
    }, [yjsState?.note]);

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
