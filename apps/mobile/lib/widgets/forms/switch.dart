import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/widgets/forms/field.dart';
import 'package:typie/widgets/tappable.dart';

class HookFormSwitch extends HookWidget {
  const HookFormSwitch({required this.name, this.initialValue, super.key});

  final String name;
  final bool? initialValue;

  @override
  Widget build(BuildContext context) {
    final controller = useAnimationController(duration: const Duration(milliseconds: 150));
    final curve = useMemoized(() => CurvedAnimation(parent: controller, curve: Curves.easeInOut));

    return HookFormField(
      name: name,
      initialValue: initialValue ?? false,
      builder: (context, field) {
        final value = field.value ?? false;

        useEffect(() {
          if (field.value ?? false) {
            unawaited(controller.forward());
          } else {
            unawaited(controller.reverse());
          }
          return null;
        }, [field.value]);

        return Tappable(
          onTap: () {
            field.value = !value;
          },
          child: Container(
            width: 44,
            height: 24,
            foregroundDecoration: BoxDecoration(
              border: Border.all(color: context.colors.borderStrong),
              borderRadius: BorderRadius.circular(4),
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(4),
              child: Stack(
                children: [
                  Row(
                    children: [
                      Expanded(child: Container(color: context.colors.accentSuccess)),
                      Expanded(child: Container(color: context.colors.surfaceMuted)),
                    ],
                  ),
                  AnimatedBuilder(
                    animation: curve,
                    builder: (context, child) {
                      return Align(
                        alignment: Alignment.lerp(Alignment.centerLeft, Alignment.centerRight, curve.value)!,
                        child: Container(
                          width: 24,
                          height: 24,
                          decoration: BoxDecoration(
                            color: context.colors.surfaceDefault,
                            border: Border(
                              left: curve.value > 0
                                  ? BorderSide(color: context.colors.borderStrong, width: curve.value)
                                  : BorderSide.none,
                              right: curve.value < 1
                                  ? BorderSide(color: context.colors.borderStrong, width: 1 - curve.value)
                                  : BorderSide.none,
                            ),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: child,
                        ),
                      );
                    },
                    child: const Icon(LucideLightIcons.check, size: 16),
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}
