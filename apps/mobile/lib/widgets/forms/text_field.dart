import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/forms/field.dart';

class HookFormTextField extends HookWidget {
  const HookFormTextField({
    required this.name,
    super.key,
    this.controller,
    this.focusNode,
    this.label,
    this.placeholder,
    this.autofocus = false,
    this.obscureText = false,
    this.keyboardType,
    this.textInputAction = TextInputAction.done,
    this.initialValue,
  });

  const factory HookFormTextField.collapsed({
    required String name,
    required TextStyle style,
    TextEditingController? controller,
    FocusNode? focusNode,
    bool autofocus,
    String? placeholder,
    String? initialValue,
    Key? key,
  }) = _HookFormCollapsedTextField;

  final String name;
  final TextEditingController? controller;
  final FocusNode? focusNode;
  final String? label;
  final String? placeholder;
  final bool autofocus;
  final bool obscureText;
  final TextInputType? keyboardType;
  final TextInputAction textInputAction;
  final String? initialValue;

  @override
  Widget build(BuildContext context) {
    final builtinController = useTextEditingController(text: initialValue);
    final builtinFocusNode = useFocusNode();

    useEffect(() {
      if (controller != null) {
        builtinController.text = controller!.text;
      }
      return null;
    }, [controller]);

    final animationController = useAnimationController(duration: const Duration(milliseconds: 150));

    final tweenedLabelColor = useMemoized(() {
      final curve = CurvedAnimation(parent: animationController, curve: Curves.ease);
      return ColorTween(begin: AppColors.gray_500, end: AppColors.gray_900).animate(curve);
    }, [animationController]);

    final tweenedBorderColor = useMemoized(() {
      final curve = CurvedAnimation(parent: animationController, curve: Curves.ease);
      return ColorTween(begin: AppColors.gray_200, end: AppColors.gray_900).animate(curve);
    }, [animationController]);

    useEffect(() {
      final currentFocusNode = focusNode ?? builtinFocusNode;

      void listener() {
        if (currentFocusNode.hasFocus) {
          animationController.forward();
        } else {
          animationController.reverse();
        }
      }

      currentFocusNode.addListener(listener);
      return () => currentFocusNode.removeListener(listener);
    }, [focusNode, builtinFocusNode]);

    useEffect(() {
      if (autofocus) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          unawaited(
            ModalRoute.of(context)!.didPush().then((value) {
              (focusNode ?? builtinFocusNode).requestFocus();
            }),
          );
        });
      }

      return null;
    }, []);

    return HookFormField(
      name: name,
      initialValue: initialValue,
      builder: (context, field) {
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (label != null) ...[
              AnimatedBuilder(
                animation: tweenedLabelColor,
                builder: (context, child) {
                  return Text(
                    label!,
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: field.error != null ? AppColors.red_600 : tweenedLabelColor.value,
                    ),
                  );
                },
              ),
              const Gap(13),
            ],
            TextField(
              controller: controller ?? builtinController,
              focusNode: focusNode ?? builtinFocusNode,
              autocorrect: false,
              obscureText: obscureText,
              keyboardType: keyboardType,
              textInputAction: textInputAction,
              decoration: InputDecoration(
                isCollapsed: true,
                border: InputBorder.none,
                hintText: placeholder,
                hintStyle: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: AppColors.gray_400),
              ),
              cursorColor: AppColors.gray_900,
              style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700),
              onChanged: (value) {
                field.value = value;
              },
              onSubmitted: (value) async {
                if (textInputAction == TextInputAction.done) {
                  await field.form.submit();
                }
              },
            ),
            const Gap(4),
            AnimatedBuilder(
              animation: tweenedBorderColor,
              builder: (context, child) {
                return Container(
                  width: double.infinity,
                  height: 1.5,
                  color: field.error != null ? AppColors.red_600 : tweenedBorderColor.value,
                );
              },
            ),
            if (field.error != null) ...[
              const Gap(6.5),
              Text(field.error!, style: const TextStyle(fontSize: 11, color: AppColors.red_600)),
            ],
          ],
        );
      },
    );
  }
}

class _HookFormCollapsedTextField extends HookFormTextField {
  const _HookFormCollapsedTextField({
    required super.name,
    required this.style,
    super.controller,
    super.focusNode,
    super.autofocus,
    super.placeholder,
    super.initialValue,
    super.key,
  });

  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    final builtinController = useTextEditingController();
    final builtinFocusNode = useFocusNode();

    final effectiveController = controller ?? builtinController;
    final effectiveFocusNode = focusNode ?? builtinFocusNode;

    useAsyncEffect(() async {
      if (autofocus) {
        await ModalRoute.of(context)!.didPush();
        effectiveFocusNode.requestFocus();
      }

      return null;
    }, [autofocus]);

    useEffect(() {
      if (initialValue != null) {
        effectiveController.text = initialValue!;
      }

      return null;
    }, [initialValue]);

    return HookFormField(
      name: name,
      initialValue: initialValue,
      builder: (context, field) {
        return TextField(
          controller: effectiveController,
          focusNode: effectiveFocusNode,
          autocorrect: false,
          obscureText: obscureText,
          keyboardType: keyboardType,
          textInputAction: textInputAction,
          decoration: InputDecoration.collapsed(
            hintText: placeholder,
            hintStyle: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: AppColors.gray_400),
          ),
          cursorColor: AppColors.gray_900,
          style: style,
          onChanged: (value) {
            field.value = value;
          },
          onSubmitted: (value) async {
            if (textInputAction == TextInputAction.done) {
              await field.form.submit();
            }
          },
        );
      },
    );
  }
}
