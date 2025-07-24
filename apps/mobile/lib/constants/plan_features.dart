import 'package:flutter/material.dart';
import 'package:typie/icons/lucide_light.dart';

class PlanFeature {
  const PlanFeature({required this.icon, required this.label});

  final IconData icon;
  final String label;
}

const basicPlanFeatures = <PlanFeature>[
  PlanFeature(icon: LucideLightIcons.book_open_text, label: '16,000자까지 작성 가능'),
  PlanFeature(icon: LucideLightIcons.images, label: '20MB까지 파일 업로드 가능'),
];

const fullPlanFeatures = <PlanFeature>[
  PlanFeature(icon: LucideLightIcons.book_open_text, label: '무제한 글자 수'),
  PlanFeature(icon: LucideLightIcons.images, label: '무제한 파일 업로드'),
  PlanFeature(icon: LucideLightIcons.spell_check, label: '맞춤법 검사'),
  PlanFeature(icon: LucideLightIcons.link, label: '커스텀 게시 주소'),
  PlanFeature(icon: LucideLightIcons.type_, label: '커스텀 폰트 업로드'),
  PlanFeature(icon: LucideLightIcons.flask_conical, label: '베타 기능 우선 접근'),
  PlanFeature(icon: LucideLightIcons.headset, label: '문제 발생 시 우선 지원'),
  PlanFeature(icon: LucideLightIcons.sprout, label: '디스코드 커뮤니티 참여'),
  PlanFeature(icon: LucideLightIcons.ellipsis, label: '그리고 더 많은 혜택'),
];
