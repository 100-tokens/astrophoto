declare module 'justified-layout' {
  export interface JustifiedLayoutBox {
    aspectRatio: number;
    top: number;
    width: number;
    height: number;
    left: number;
    forcedAspectRatio?: boolean;
  }
  export interface JustifiedLayoutResult {
    containerHeight: number;
    widowCount: number;
    boxes: JustifiedLayoutBox[];
  }
  export interface JustifiedLayoutOptions {
    containerWidth?: number;
    containerPadding?: number | { top?: number; right?: number; bottom?: number; left?: number };
    boxSpacing?: number | { horizontal?: number; vertical?: number };
    targetRowHeight?: number;
    targetRowHeightTolerance?: number;
    maxNumRows?: number;
    forceAspectRatio?: boolean | number;
    showWidows?: boolean;
    fullWidthBreakoutRowCadence?: boolean | number;
  }
  export default function justifiedLayout(
    aspectRatios: number[] | Array<{ aspectRatio: number }>,
    options?: JustifiedLayoutOptions
  ): JustifiedLayoutResult;
}
