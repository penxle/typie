import 'package:flutter/material.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/screens/native_editor/state/state.dart';

class PasteOptionWidget extends StatelessWidget {
  const PasteOptionWidget({required this.controller, required this.info, super.key});

  final EditorController controller;
  final PasteOptionsInfo info;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      top: false,
      child: Padding(
        padding: const EdgeInsets.only(bottom: 20),
        child: Center(
          child: GestureDetector(
            onTap: () {
              controller
                ..dispatch({
                  'type': 'setSelection',
                  'anchorNodeId': info.from['nodeId'],
                  'anchorOffset': info.from['offset'],
                  'anchorAffinity': info.from['affinity'],
                  'headNodeId': info.to['nodeId'],
                  'headOffset': info.to['offset'],
                  'headAffinity': info.to['affinity'],
                })
                ..dispatch({'type': 'pasteText', 'text': info.text})
                ..updateState((s) => s.copyWith(pasteOptions: null));
            },
            behavior: HitTestBehavior.opaque,
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
              decoration: BoxDecoration(
                color: context.colors.surfaceDefault,
                borderRadius: BorderRadius.circular(100),
                boxShadow: [
                  BoxShadow(color: Colors.black.withValues(alpha: 0.08), offset: const Offset(0, 2), blurRadius: 8),
                ],
                border: Border.all(color: context.colors.borderStrong),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(LucideLightIcons.clipboard_type, size: 18, color: context.colors.textSubtle),
                  const SizedBox(width: 8),
                  Text(
                    '서식 없이 다시 붙여넣기',
                    style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textDefault),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
