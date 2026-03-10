import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/widgets/forms/field.dart';
import 'package:typie/widgets/popover/list.dart';
import 'package:typie/widgets/popover/popover.dart';

class HookFormSelect<T> extends StatelessWidget {
  const HookFormSelect({
    required this.name,
    required this.initialValue,
    required this.items,
    this.values,
    this.position = PopoverPosition.bottomRight,
    super.key,
  });

  final String name;
  final T initialValue;
  final List<T>? values;
  final List<HookFormSelectItem<T>> items;
  final PopoverPosition position;

  @override
  Widget build(BuildContext context) {
    return HookFormField(
      name: name,
      initialValue: initialValue,
      builder: (context, field) {
        final selectedValues = values ?? [field.value as T];
        final isIndeterminate = selectedValues.toSet().length > 1;

        final item = isIndeterminate
            ? HookFormSelectItem<T>(
                icon: LucideLightIcons.minus,
                label: selectedValues
                    .map((v) => items.firstWhereOrNull((item) => item.value == v)?.label ?? '')
                    .join(', '),
                value: field.value as T,
              )
            : items.firstWhereOrNull((item) => item.value == field.value);

        return Popover(
          position: position,
          maxWidth: 320,
          matchAnchorWidth: false,
          screenPadding: const EdgeInsets.all(20),
          collapsedBorderRadius: BorderRadius.circular(8),
          expandedBorderRadius: BorderRadius.circular(16),
          backgroundColor: context.colors.surfaceDefault,
          borderSide: BorderSide(color: context.colors.borderStrong),
          anchor: _SelectAnchor(item: item),
          pane: _SelectPane(
            items: items,
            selectedValues: selectedValues,
            onSelected: (value) {
              field.value = value;
            },
          ),
        );
      },
    );
  }
}

class _SelectAnchor<T> extends StatelessWidget {
  const _SelectAnchor({required this.item});

  final HookFormSelectItem<T>? item;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: ShapeDecoration(
        color: context.colors.surfaceDefault,
        shape: RoundedSuperellipseBorder(
          borderRadius: BorderRadius.circular(8),
          side: BorderSide(color: context.colors.borderStrong),
        ),
      ),
      child: Padding(
        padding: EdgeInsets.zero,
        child: _SelectAnchorBody(item: item),
      ),
    );
  }
}

class _SelectAnchorBody<T> extends StatelessWidget {
  const _SelectAnchorBody({required this.item});

  final HookFormSelectItem<T>? item;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      child: _SelectAnchorContent(item: item),
    );
  }
}

class _SelectAnchorContent<T> extends StatelessWidget {
  const _SelectAnchorContent({required this.item});

  final HookFormSelectItem<T>? item;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        if (item?.icon != null) ...[Icon(item!.icon, size: 18, color: context.colors.textSubtle), const Gap(4)],
        Text(
          item?.label ?? '(알 수 없음)',
          style: TextStyle(fontSize: 16, height: 1, color: context.colors.textSubtle),
          strutStyle: const StrutStyle(fontSize: 16, height: 1, leading: 0, forceStrutHeight: true),
        ),
        const Gap(8),
        Icon(LucideLightIcons.chevron_down, size: 16, color: context.colors.textFaint),
      ],
    );
  }
}

class _SelectPopoverItem<T> extends StatelessWidget {
  const _SelectPopoverItem({required this.item, required this.isSelected});

  final HookFormSelectItem<T> item;
  final bool isSelected;

  @override
  Widget build(BuildContext context) {
    final labelColor = isSelected ? context.colors.textDefault : context.colors.textSubtle;
    final secondaryColor = isSelected ? context.colors.textMuted : context.colors.textFaint;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      child: Row(
        crossAxisAlignment: item.description == null ? CrossAxisAlignment.center : CrossAxisAlignment.start,
        spacing: 10,
        children: [
          if (item.icon != null) Icon(item.icon, size: 18, color: labelColor),
          Expanded(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 2,
              children: [
                Text(
                  item.label,
                  style: TextStyle(fontSize: 16, height: 1, color: labelColor),
                  strutStyle: const StrutStyle(fontSize: 16, height: 1, leading: 0, forceStrutHeight: true),
                ),
                if (item.description != null)
                  Text(item.description!, style: TextStyle(fontSize: 15, color: secondaryColor)),
              ],
            ),
          ),
          SizedBox.square(
            dimension: 16,
            child: isSelected ? Icon(LucideLightIcons.check, size: 16, color: context.colors.textDefault) : null,
          ),
        ],
      ),
    );
  }
}

class _SelectPane<T> extends StatelessWidget {
  const _SelectPane({required this.items, required this.selectedValues, required this.onSelected});

  final List<HookFormSelectItem<T>> items;
  final List<T> selectedValues;
  final ValueChanged<T> onSelected;

  @override
  Widget build(BuildContext context) {
    return IntrinsicWidth(
      child: PopoverList(
        items: [
          for (final item in items)
            PopoverListItem(
              onSelected: () {
                onSelected(item.value);
                Popover.close(context);
              },
              child: _SelectPopoverItem(item: item, isSelected: selectedValues.contains(item.value)),
            ),
        ],
      ),
    );
  }
}

class HookFormSelectItem<T> {
  const HookFormSelectItem({required this.label, required this.value, this.icon, this.description});

  final IconData? icon;
  final String label;
  final String? description;
  final T value;
}
