import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class OssLicensesScreen extends HookWidget {
  const OssLicensesScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final future = useMemoized(() {
      return LicenseRegistry.licenses.fold(<LicenseEntry>[], (prev, license) => prev..add(license)).then((entries) {
        final licenses = <String, List<String>>{};
        for (final entry in entries) {
          for (final package in entry.packages) {
            licenses.putIfAbsent(package, () => []).addAll(entry.paragraphs.map((v) => v.text));
          }
        }
        return licenses.entries.toList()..sort((a, b) => a.key.compareTo(b.key));
      });
    });

    final licenses = useFuture(future);

    return Screen(
      heading: const Heading(title: '오픈소스 라이센스'),
      child: licenses.hasData
          ? ListView.separated(
              physics: const AlwaysScrollableScrollPhysics(),
              padding: Pad(all: 20, bottom: MediaQuery.paddingOf(context).bottom),
              itemCount: licenses.data!.length,
              itemBuilder: (context, index) {
                final license = licenses.data![index];

                return Tappable(
                  onTap: () async {
                    await context.showBottomSheet(
                      child: AppFullBottomSheet(
                        title: license.key,
                        padding: Pad.zero,
                        child: ListView.separated(
                          physics: const AlwaysScrollableScrollPhysics(),
                          padding: Pad(all: 20, bottom: MediaQuery.paddingOf(context).bottom),
                          itemCount: license.value.length,
                          itemBuilder: (context, index) {
                            return Text(license.value[index], style: const TextStyle(fontSize: 14));
                          },
                          separatorBuilder: (context, index) {
                            return const Gap(12);
                          },
                        ),
                      ),
                    );
                  },
                  child: Container(
                    decoration: BoxDecoration(
                      color: AppColors.white,
                      border: Border.all(color: AppColors.gray_950),
                      borderRadius: const BorderRadius.all(Radius.circular(8)),
                    ),
                    padding: const Pad(horizontal: 16, vertical: 12),
                    child: Row(
                      children: [
                        Expanded(
                          child: Text(
                            license.key,
                            style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                            overflow: TextOverflow.ellipsis,
                            maxLines: 1,
                          ),
                        ),
                        const Icon(LucideLightIcons.chevron_right, size: 16),
                      ],
                    ),
                  ),
                );
              },
              separatorBuilder: (context, index) {
                return const Gap(12);
              },
            )
          : const Center(child: CircularProgressIndicator()),
    );
  }
}
