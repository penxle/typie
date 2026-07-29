import GenerationView from './GenerationView.svelte';
import ItemControl from './ItemControl.svelte';
import RunControl from './RunControl.svelte';
import RunReview from './RunReview.svelte';
import Summary from './Summary.svelte';

// 이 세대는 단계 산출물을 남기지 않는다 — 열람 화면의 산출물 버튼도 뜨지 않는다.
export const UI = { GenerationView, ItemControl, RunControl, RunReview, Summary };
