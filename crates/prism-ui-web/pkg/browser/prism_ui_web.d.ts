/* tslint:disable */
/* eslint-disable */

declare class PrismWebRenderer {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    render(frame_uniform_bytes: Uint8Array, optical_paths: Float32Array, width: number, height: number, light_resolution_scale: number, material_resolution_scale: number, light_source_sample_count: number, material_source_sample_count: number, scissor_x: number, scissor_y: number, scissor_width: number, scissor_height: number, render_light: boolean, hdr_headroom: number): void;
    renderComputed(frame_uniform_bytes: Uint8Array, planes: Float32Array, plane_count: number, transition_scale: number, prism_scale: number, bevel: number, phase: number, light_phase: number, depth_transition: number, perspective_transition: number, light_count: number, light_radius: number, source_size: number, source_divergence: number, source_sample_count: number, ior: number, dispersion: number, width: number, height: number, light_resolution_scale: number, material_resolution_scale: number, light_source_sample_count: number, material_source_sample_count: number, scissor_x: number, scissor_y: number, scissor_width: number, scissor_height: number, render_light: boolean, hdr_headroom: number): void;
    readonly frameUniformByteLength: number;
}

declare class PrismWebRuntime {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    static create(canvas: HTMLCanvasElement): Promise<PrismWebRuntime>;
    createRenderer(canvas: HTMLCanvasElement, prefer_hdr: boolean): PrismWebRenderer;
}

export type { PrismWebRenderer, PrismWebRuntime };

export function createInstance(wasmModule: WebAssembly.Module): Promise<{
    PrismWebRenderer: typeof PrismWebRenderer;
    PrismWebRuntime: typeof PrismWebRuntime;
}>;
