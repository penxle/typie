import 'dart:async';
import 'dart:ui' as ui;

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/hooks.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/site/__generated__/create_site_mutation.req.gql.dart';
import 'package:typie/screens/site/__generated__/screen_query.data.gql.dart';
import 'package:typie/screens/site/__generated__/screen_query.req.gql.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/plan_upgrade_bottom_sheet.dart';
import 'package:typie/widgets/popover/list.dart';
import 'package:typie/widgets/popover/popover.dart';

const _spacePopoverScreenPadding = EdgeInsets.fromLTRB(20, 8, 20, 8);
const _spaceButtonRadius = 14.0;
const _spaceLogoInset = 9.0;
const _spaceLogoSize = 26.0;
const _spaceLogoRadius = 6.0;

class SpacePopoverButton extends HookWidget {
  const SpacePopoverButton({
    required this.backgroundColor,
    required this.boxShadow,
    required this.via,
    this.siteName,
    this.siteLogoUrl,
    this.enabled = true,
    super.key,
  });

  final String? siteName;
  final String? siteLogoUrl;
  final Color backgroundColor;
  final List<BoxShadow> boxShadow;
  final String via;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final site = useService<Site>();
    final siteId = useValueListenable(site);
    final data = useQuery(GSiteScreen_QueryReq((b) => b.vars.siteId = siteId));

    final currentSiteName = data?.site.name ?? siteName;
    final currentSiteLogoUrl = data?.site.logo.url ?? siteLogoUrl;
    final anchor = _SpacePopoverAnchorButton(
      logoUrl: currentSiteLogoUrl,
      backgroundColor: backgroundColor,
      boxShadow: boxShadow,
    );

    if (!enabled) {
      return anchor;
    }

    return Popover(
      position: PopoverPosition.bottomLeft,
      screenPadding: _spacePopoverScreenPadding,
      collapsedBorderRadius: BorderRadius.circular(_spaceButtonRadius),
      backgroundColor: backgroundColor,
      borderSide: BorderSide(color: context.colors.borderStrong),
      anchor: anchor,
      pane: _SpacePopoverPane(siteName: currentSiteName, siteLogoUrl: currentSiteLogoUrl, data: data, via: via),
    );
  }
}

class _SpacePopoverAnchorButton extends StatelessWidget {
  const _SpacePopoverAnchorButton({required this.backgroundColor, required this.boxShadow, this.logoUrl});

  final String? logoUrl;
  final Color backgroundColor;
  final List<BoxShadow> boxShadow;

  @override
  Widget build(BuildContext context) {
    final borderRadius = BorderRadius.circular(_spaceButtonRadius);

    return SizedBox(
      width: HeadingCircleButton.controlSize,
      height: HeadingCircleButton.controlSize,
      child: DecoratedBox(
        decoration: ShapeDecoration(
          color: backgroundColor,
          shadows: boxShadow,
          shape: RoundedSuperellipseBorder(
            borderRadius: borderRadius,
            side: BorderSide(color: context.colors.borderStrong),
          ),
        ),
        child: ClipRSuperellipse(
          borderRadius: borderRadius,
          child: Center(
            child: logoUrl == null
                ? Icon(LucideLightIcons.folder_open, size: 18, color: context.colors.textDefault)
                : Padding(
                    padding: const EdgeInsets.all(_spaceLogoInset),
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(_spaceLogoRadius),
                      child: CachedNetworkImage(
                        imageUrl: logoUrl!,
                        width: _spaceLogoSize,
                        height: _spaceLogoSize,
                        fit: BoxFit.cover,
                      ),
                    ),
                  ),
          ),
        ),
      ),
    );
  }
}

class _SpacePopoverPane extends HookWidget {
  const _SpacePopoverPane({required this.via, this.siteName, this.siteLogoUrl, this.data});

  final String via;
  final String? siteName;
  final String? siteLogoUrl;
  final GSiteScreen_QueryData? data;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final site = useService<Site>();
    final mixpanel = useService<Mixpanel>();

    final currentSiteId = data?.site.id ?? site.siteId;
    final otherSites = data?.me?.sites.where((otherSite) => otherSite.id != currentSiteId).toList() ?? [];
    final hasSubscription = data?.me?.subscription != null;
    final isLoading = data == null;

    Future<void> openSiteSettings() async {
      unawaited(mixpanel.track('open_site_settings', properties: {'via': via}));
      await context.router.push(const SiteSettingsRoute());
    }

    Future<void> handleAddSite() async {
      if (isLoading) {
        return;
      }

      if (!hasSubscription) {
        final result = await context.showBottomSheet<PlanUpgradeResult>(
          child: const PlanUpgradeBottomSheet(message: '멀티 스페이스는 FULL ACCESS 플랜에서 사용할 수 있어요.'),
        );

        if (result == PlanUpgradeResult.upgrade && context.mounted) {
          await context.router.push(const EnrollPlanRoute());
        }
        return;
      }

      await context.showBottomSheet(
        child: _CreateSiteBottomSheet(client: client, site: site, mixpanel: mixpanel, via: via),
      );
    }

    void selectOtherSite(String siteId) {
      Popover.close(context);
      site.setSiteId(siteId);
    }

    return IntrinsicWidth(
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: 280, maxHeight: MediaQuery.sizeOf(context).height * 0.65),
        child: SingleChildScrollView(
          primary: false,
          padding: const EdgeInsets.all(Popover.panePadding),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _SpacePopoverHeader(siteName: siteName, siteLogoUrl: siteLogoUrl),
              const SizedBox(height: 4),
              PopoverList(
                indicatorColor: context.colors.surfaceMuted,
                items: [
                  PopoverListItem(
                    onSelected: () {
                      Popover.close(context);
                      unawaited(openSiteSettings());
                    },
                    child: const _SpacePopoverItem(icon: LucideLightIcons.settings, label: '스페이스 설정'),
                  ),
                  PopoverListItem(
                    onSelected: () {
                      Popover.close(context);
                      unawaited(context.router.push(TrashRoute()));
                    },
                    child: const _SpacePopoverItem(icon: LucideLightIcons.trash_2, label: '휴지통'),
                  ),
                ],
              ),
              const SizedBox(height: 12),
              HorizontalDivider(color: context.colors.borderSubtle),
              const SizedBox(height: 12),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8),
                child: Text(
                  '다른 스페이스',
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 0.3,
                    color: context.colors.textFaint,
                  ),
                ),
              ),
              const SizedBox(height: 8),
              if (isLoading)
                SizedBox(
                  height: 48,
                  child: Center(
                    child: SizedBox(
                      width: 18,
                      height: 18,
                      child: CircularProgressIndicator(strokeWidth: 2, color: context.colors.textSubtle),
                    ),
                  ),
                )
              else
                PopoverList(
                  indicatorColor: context.colors.surfaceMuted,
                  items: [
                    for (final otherSite in otherSites)
                      PopoverListItem(
                        key: ValueKey(otherSite.id),
                        onSelected: () {
                          selectOtherSite(otherSite.id);
                        },
                        child: _SpacePopoverSpaceItem(site: otherSite),
                      ),
                    PopoverListItem(
                      onSelected: () {
                        Popover.close(context);
                        unawaited(handleAddSite());
                      },
                      child: const _SpacePopoverItem(icon: LucideLightIcons.plus, label: '새 스페이스 생성'),
                    ),
                  ],
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _SpacePopoverHeader extends StatelessWidget {
  const _SpacePopoverHeader({this.siteName, this.siteLogoUrl});

  static const _targetLogoLeft = 8.0;
  static const _targetNameGap = 12.0;

  final String? siteName;
  final String? siteLogoUrl;

  @override
  Widget build(BuildContext context) {
    final transition = PopoverPaneTransitionScope.maybeOf(context);
    final progress = (transition?.progress ?? 1).clamp(0.0, 1.0);
    final anchorContentRect =
        transition?.anchorContentRect ??
        const Rect.fromLTWH(0, 0, HeadingCircleButton.controlSize, HeadingCircleButton.controlSize);
    final sourceLeft = anchorContentRect.left + _spaceLogoInset - Popover.panePadding;
    final sourceTop = anchorContentRect.top + (anchorContentRect.height - _spaceLogoSize) / 2 - Popover.panePadding;
    const targetTop = (HeadingCircleButton.controlSize - _spaceLogoSize) / 2;
    final logoLeft = ui.lerpDouble(sourceLeft, _targetLogoLeft, progress)!;
    final logoTop = ui.lerpDouble(sourceTop, targetTop, progress)!;
    final logoSize = ui.lerpDouble(_spaceLogoSize, _spaceLogoSize, progress)!;
    const textLeft = _targetLogoLeft + _spaceLogoSize + _targetNameGap;

    return SizedBox(
      height: HeadingCircleButton.controlSize,
      child: Stack(
        children: [
          Padding(
            padding: const EdgeInsets.only(left: textLeft, right: 16),
            child: Align(
              alignment: Alignment.centerLeft,
              child: Text(
                siteName ?? '내 스페이스',
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w600),
              ),
            ),
          ),
          Positioned(
            left: logoLeft,
            top: logoTop,
            width: logoSize,
            height: logoSize,
            child: _SpacePopoverLogo(logoUrl: siteLogoUrl, size: logoSize),
          ),
        ],
      ),
    );
  }
}

class _SpacePopoverLogo extends StatelessWidget {
  const _SpacePopoverLogo({this.logoUrl, this.size = _spaceLogoSize});

  final String? logoUrl;
  final double size;

  @override
  Widget build(BuildContext context) {
    if (logoUrl == null) {
      return SizedBox(
        width: size,
        height: size,
        child: Center(
          child: Icon(LucideLightIcons.folder_open, size: size * 0.75, color: context.colors.textDefault),
        ),
      );
    }

    return ClipRRect(
      borderRadius: BorderRadius.circular(_spaceLogoRadius),
      child: CachedNetworkImage(imageUrl: logoUrl!, width: size, height: size, fit: BoxFit.cover),
    );
  }
}

class _SpacePopoverSpaceItem extends StatelessWidget {
  const _SpacePopoverSpaceItem({required this.site});

  final GSiteScreen_QueryData_me_sites site;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 48,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          spacing: 12,
          children: [
            _SpacePopoverLogo(logoUrl: site.logo.url, size: 28),
            Expanded(
              child: Text(
                site.name,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w500),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _SpacePopoverItem extends StatelessWidget {
  const _SpacePopoverItem({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 42,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          spacing: 12,
          children: [
            Icon(icon, size: 18, color: context.colors.textDefault),
            Expanded(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textDefault),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _CreateSiteBottomSheet extends HookWidget {
  const _CreateSiteBottomSheet({required this.client, required this.site, required this.mixpanel, required this.via});

  final GraphQLClient client;
  final Site site;
  final Mixpanel mixpanel;
  final String via;

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController();

    return ConfirmBottomSheet(
      title: '새 스페이스 생성',
      message: '스페이스는 독립된 글쓰기 공간이에요.\n주제나 목적에 따라 글을 나누어 관리해보세요.',
      confirmText: '생성',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 6,
        children: [
          Text(
            '스페이스 이름',
            style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textDefault),
          ),
          TextField(
            controller: controller,
            autofocus: true,
            decoration: InputDecoration(
              hintText: '새 스페이스',
              hintStyle: TextStyle(color: context.colors.textDisabled),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
              contentPadding: const Pad(horizontal: 12, vertical: 10),
            ),
            style: const TextStyle(fontSize: 16),
          ),
        ],
      ),
      onConfirm: () async {
        final name = controller.text.trim();

        try {
          await context.runWithLoader(() async {
            final result = await client.request(
              GSiteScreen_CreateSite_MutationReq((b) => b..vars.input.name = name.isEmpty ? '새 스페이스' : name),
            );

            site.setSiteId(result.createSite.id);
          });

          unawaited(mixpanel.track('create_site', properties: {'via': via}));

          if (context.mounted) {
            context.toast(ToastType.success, '새 스페이스가 생성되었어요.');
          }
        } catch (_) {
          if (context.mounted) {
            context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
          }
        }
      },
    );
  }
}
