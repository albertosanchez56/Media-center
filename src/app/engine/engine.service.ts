import { Injectable, signal } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { describeError } from '../shared/describe-error';

/** Debe coincidir con `STATE_EVENT` en `src-tauri/src/engine/state.rs`. */
const STATE_EVENT = 'engine-state';
/** Debe coincidir con `WHEEL_EVENT` en `src-tauri/src/engine/surface.rs`. */
const WHEEL_EVENT = 'engine-surface-wheel';

/** Integraciones evaluadas en DP-01: A (libmpv en el proceso) y B (proceso `mpv.exe`). */
export type EngineKind = 'libmpv' | 'mpv-process';

export interface EngineState {
  kind: EngineKind | null;
  path: string | null;
  associatedVideo: string | null;
  timePos: number | null;
  duration: number | null;
  pause: boolean;
  avsync: number | null;
  eofReached: boolean;
  startFileCount: number;
  fileLoadedCount: number;
  processId: number | null;
  mpvVersion: string | null;
  ffmpegVersion: string | null;
  lgplBuild: boolean | null;
  lastError: string | null;
}

export interface OpenRequest {
  kind: EngineKind;
  path: string;
  associatedVideo: string | null;
  d3d11Flip: boolean;
}

/** Píxeles físicos relativos al área cliente de la ventana principal. */
export interface SurfaceRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** El hueco completo y la parte que deja ver la interfaz, ambos en `SurfaceRect`. */
export interface SurfacePlacement {
  bounds: SurfaceRect;
  visible: SurfaceRect;
}

/** Giro de la rueda sobre el vídeo, en unidades de Windows (120 por paso; positivo hacia arriba). */
export interface SurfaceWheel {
  delta: number;
  horizontal: boolean;
}

export interface ClientPoint {
  x: number;
  y: number;
}

export interface TrackInfo {
  id: number | null;
  kind: string;
  selected: boolean;
  external: boolean;
  externalFilename: string | null;
  albumart: boolean;
  title: string | null;
  codec: string | null;
}

export interface PropertyReading {
  name: string;
  value: string | null;
}

export interface TrackReport {
  tracks: TrackInfo[];
  audioOnlyFromMainFile: boolean;
  selectedExternalVideo: string | null;
  playback: PropertyReading[];
}

export interface WindowInfo {
  handle: string;
  className: string;
  title: string;
  processId: number;
  visible: boolean;
  enabled: boolean;
  insideMainWindow: boolean;
  rect: SurfaceRect;
}

export interface HitTest {
  point: ClientPoint;
  className: string;
  hitsVideo: boolean;
}

export interface WindowReport {
  surfaceVisible: boolean;
  surfaceRect: SurfaceRect;
  strayWindows: WindowInfo[];
  videoWindows: WindowInfo[];
  region: SurfaceRect[];
  hitTests: HitTest[];
}

export const IDLE_ENGINE_STATE: EngineState = {
  kind: null,
  path: null,
  associatedVideo: null,
  timePos: null,
  duration: null,
  pause: false,
  avsync: null,
  eofReached: false,
  startFileCount: 0,
  fileLoadedCount: 0,
  processId: null,
  mpvVersion: null,
  ffmpegVersion: null,
  lgplBuild: null,
  lastError: null,
};

/** Acceso al motor experimental de F1.4. Los comandos solo existen en compilaciones de desarrollo. */
@Injectable({ providedIn: 'root' })
export class EngineService {
  readonly state = signal<EngineState>(IDLE_ENGINE_STATE);
  /** Último fallo al colocar la superficie; esas llamadas no tienen a quién devolver el error. */
  readonly surfaceError = signal<string | null>(null);

  private listening: Promise<void> | null = null;

  /** Empieza a recibir el estado que emite Rust. Solo se suscribe una vez. */
  connect(): Promise<void> {
    this.listening ??= listen<EngineState>(STATE_EVENT, (event) => this.state.set(event.payload))
      .then(() => undefined)
      .catch((error: unknown) => {
        this.listening = null;
        throw new Error(describeError(error));
      });
    return this.listening;
  }

  async open(request: OpenRequest): Promise<void> {
    this.state.set(await call<EngineState>('engine_open', { request }));
  }

  setPause(paused: boolean): Promise<void> {
    return call('engine_set_pause', { paused });
  }

  seek(seconds: number): Promise<void> {
    return call('engine_seek', { seconds });
  }

  async stop(): Promise<void> {
    await call('engine_stop');
    this.state.set(IDLE_ENGINE_STATE);
  }

  async refresh(): Promise<void> {
    this.state.set(await call<EngineState>('engine_status'));
  }

  tracks(): Promise<TrackReport> {
    return call('engine_tracks');
  }

  windowReport(points: ClientPoint[]): Promise<WindowReport> {
    return call('engine_window_report', { points });
  }

  /** `null` oculta la superficie: la interfaz no deja ver nada del hueco. */
  async setSurface(placement: SurfacePlacement | null): Promise<void> {
    await this.reportSurface(call('engine_set_surface', { placement }));
  }

  /** La rueda sobre el vídeo no llega a la página; Rust la reenvía con este evento. */
  onSurfaceWheel(handler: (wheel: SurfaceWheel) => void): Promise<() => void> {
    return listen<SurfaceWheel>(WHEEL_EVENT, (event) => handler(event.payload));
  }

  async setHoles(holes: SurfaceRect[]): Promise<void> {
    await this.reportSurface(call('engine_set_holes', { holes }));
  }

  private async reportSurface(request: Promise<void>): Promise<void> {
    try {
      await request;
      this.surfaceError.set(null);
    } catch (error) {
      this.surfaceError.set(describeError(error));
    }
  }
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw new Error(describeError(error));
  }
}
