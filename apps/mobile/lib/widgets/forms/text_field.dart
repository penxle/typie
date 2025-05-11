import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/forms/field.dart';

class FormTextField extends HookWidget {
  const FormTextField({
    required this.name,
    super.key,
    this.controller,
    this.focusNode,
    this.labelText,
    this.hintText,
    this.autofocus = false,
    this.obscureText = false,
    this.keyboardType,
    this.textInputAction,
    this.initialValue,
    this.validators,
  });

  final String name;
  final TextEditingController? controller;
  final FocusNode? focusNode;
  final String? labelText;
  final String? hintText;
  final bool autofocus;
  final bool obscureText;
  final TextInputType? keyboardType;
  final TextInputAction? textInputAction;
  final String? initialValue;
  final List<FormFieldValidator<String>>? validators;

  @override
  Widget build(BuildContext context) {
    final textController = useTextEditingController(text: initialValue);
    final textFocusNode = useFocusNode();

    useEffect(() {
      if (controller != null) {
        textController.text = controller!.text;
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
      final currentFocusNode = focusNode ?? textFocusNode;

      void listener() {
        if (currentFocusNode.hasFocus) {
          animationController.forward();
        } else {
          animationController.reverse();
        }
      }

      currentFocusNode.addListener(listener);
      return () => currentFocusNode.removeListener(listener);
    }, [focusNode, textFocusNode]);

    useEffect(() {
      if (autofocus) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          unawaited(
            ModalRoute.of(context)!.didPush().then((value) {
              (focusNode ?? textFocusNode).requestFocus();
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
            if (labelText != null) ...[
              AnimatedBuilder(
                animation: tweenedLabelColor,
                builder: (context, child) {
                  return Text(
                    labelText!,
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: field.error != null ? AppColors.red_600 : tweenedLabelColor.value,
                    ),
                  );
                },
              ),
              const Box.gap(13),
            ],
            TextField(
              controller: controller ?? textController,
              focusNode: focusNode ?? textFocusNode,
              autocorrect: false,
              obscureText: obscureText,
              keyboardType: keyboardType,
              textInputAction: textInputAction,
              decoration: InputDecoration(
                isCollapsed: true,
                border: InputBorder.none,
                hintText: hintText,
                hintStyle: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: AppColors.gray_400),
              ),
              cursorColor: AppColors.gray_900,
              style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700),
              onChanged: (value) {
                field.value = value;
              },
              onSubmitted: (value) {
                if (textInputAction == TextInputAction.done) {
                  field.form.submit();
                }
              },
            ),
            const Box.gap(4),
            AnimatedBuilder(
              animation: tweenedBorderColor,
              builder: (context, child) {
                return Box(
                  width: double.infinity,
                  height: 1.5,
                  color: field.error != null ? AppColors.red_600 : tweenedBorderColor.value,
                );
              },
            ),
            if (field.error != null) ...[
              const Box.gap(6.5),
              Text(field.error!, style: const TextStyle(fontSize: 11, color: AppColors.red_600)),
            ],
          ],
        );
      },
    );
  }
}
