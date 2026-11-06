import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  computed,
  inject,
  signal,
} from '@angular/core';
import {
  ClientPoint,
  EngineKind,
  EngineService,
  SurfaceRect,
  TrackInfo,
  TrackReport,
  TrackSelection,
  WindowReport,
} from '../../engine/engine.service';
import { VideoSurfaceDirective, toPhysicalRect } from '../../engine/video-surface.directive';
import { describeError } from '../../shared/describe-error';

export type StageSize = 'small' | 'medium' | 'expanded';

/** Huecos entre los que se traslada la única superficie de vídeo. */
export type VideoSlot = 'main' | 'mini';

/** `window` solo pone la ventana a pantalla completa; `video` además hace que el vídeo la ocupe. */
export type FullscreenMode = 'off' | 'window' | 'video';

export const ENGINE_OPTIONS: readonly { kind: EngineKind; label: string }[] = [
  { kind: 'libmpv', label: 'A · libmpv dentro de la aplicación' },
  { kind: 'mpv-process', label: 'B · proceso mpv.exe' },
];

export const STAGE_SIZES: readonly { size: StageSize; label: string }[] = [
  { size: 'small', label: 'Pequeño' },
  { size: 'medium', label: 'Mediano' },
  { size: 'expanded', label: 'Ampliado' },
];

export const VIDEO_SLOTS: readonly { slot: VideoSlot; label: string }[] = [
  { slot: 'main', label: 'Hueco principal' },
  { slot: 'mini', label: 'Minirreproductor' },
];

export const MENU_ITEMS = ['Primera opción', 'Segunda opción', 'Tercera opción'];

/** Quita espacios y las comillas que añade «Copiar como ruta de acceso» del Explorador. */
export function cleanPath(value: string): string {
  const trimmed = value.trim();
  return trimmed.length >= 2 && trimmed.startsWith('"') && trimmed.endsWith('"')
    ? trimmed.slice(1, -1).trim()
    : trimmed;
}

/** Banco de pruebas de F1.4: compara las integraciones A y B de DP-01 sin decidir entre ellas. */
@Component({
  selector: 'app-engine-diagnostics',
  imports: [VideoSurfaceDirective],
  templateUrl: './engine-diagnostics.component.html',
  styleUrl: './engine-diagnostics.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '(document:keydown.escape)': 'exitFullscreen()' },
})
export class EngineDiagnosticsComponent {
  private readonly engine = inject(EngineService);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;

  protected readonly engineOptions = ENGINE_OPTIONS;
  protected readonly stageSizes = STAGE_SIZES;
  protected readonly slots = VIDEO_SLOTS;
  protected readonly menuItems = MENU_ITEMS;

  protected readonly state = this.engine.state;
  protected readonly surfaceError = this.engine.surfaceError;
  protected readonly active = computed(() => this.state().kind !== null);

  protected readonly kind = signal<EngineKind>('libmpv');
  protected readonly path = signal('');
  protected readonly associatedVideo = signal('');
  protected readonly d3d11Flip = signal(false);
  protected readonly subtitlePath = signal('');

  protected readonly size = signal<StageSize>('medium');
  protected readonly slot = signal<VideoSlot>('main');
  protected readonly fullscreen = signal<FullscreenMode>('off');
  protected readonly engineFullscreen = signal(false);
  protected readonly stageSize = computed(() =>
    this.fullscreen() === 'video' ? 'fullscreen' : this.size(),
  );
  protected readonly overlay = signal(false);
  protected readonly clip = signal(false);
  protected readonly menuOpen = signal(false);
  protected readonly overlayClicks = signal(0);
  protected readonly menuChoice = signal<string | null>(null);
  protected readonly keyCount = signal(0);

  protected readonly busy = signal(false);
  protected readonly error = signal<string | null>(null);
  protected readonly tracks = signal<TrackReport | null>(null);
  protected readonly report = signal<WindowReport | null>(null);

  constructor() {
    this.engine.connect().catch((error: unknown) => this.error.set(describeError(error)));
    inject(DestroyRef).onDestroy(() => {
      if (this.fullscreen() !== 'off') {
        this.engine.setWindowFullscreen(false).catch(() => undefined);
      }
      if (this.active()) {
        this.engine.stop().catch(() => undefined);
      }
    });
  }

  protected readonly mainPath = computed(() => cleanPath(this.path()));
  protected readonly associatedPath = computed(() => cleanPath(this.associatedVideo()));
  protected readonly cleanSubtitlePath = computed(() => cleanPath(this.subtitlePath()));

  protected open(): Promise<void> {
    const associated = this.associatedPath();
    return this.run(async () => {
      this.tracks.set(null);
      await this.engine.open({
        kind: this.kind(),
        path: this.mainPath(),
        associatedVideo: associated === '' ? null : associated,
        d3d11Flip: this.d3d11Flip(),
      });
    });
  }

  protected togglePause(): Promise<void> {
    return this.run(() => this.engine.setPause(!this.state().pause));
  }

  protected seek(seconds: number): Promise<void> {
    return this.run(() => this.engine.seek(seconds));
  }

  protected stop(): Promise<void> {
    return this.run(() => this.engine.stop());
  }

  protected readTracks(): Promise<void> {
    return this.run(async () => this.tracks.set(await this.engine.tracks()));
  }

  /** Tipo de pista que se puede activar desde la tabla; `null` si ya está activa o no se elige. */
  protected selectable(track: TrackInfo): TrackSelection | null {
    if (track.selected || track.id === null) {
      return null;
    }
    return track.kind === 'audio' ? 'audio' : track.kind === 'sub' ? 'subtitle' : null;
  }

  protected useTrack(kind: TrackSelection, id: number | null): Promise<void> {
    return this.run(async () => {
      await this.engine.selectTrack(kind, id);
      this.tracks.set(await this.engine.tracks());
    });
  }

  protected addSubtitle(): Promise<void> {
    const path = this.cleanSubtitlePath();
    return this.run(async () => {
      await this.engine.addSubtitle(path);
      this.tracks.set(await this.engine.tracks());
    });
  }

  protected toggleWindowFullscreen(): Promise<void> {
    const enable = this.fullscreen() === 'off';
    return this.run(async () => {
      const result = await this.engine.setWindowFullscreen(enable);
      this.fullscreen.set(result ? 'window' : 'off');
    });
  }

  /** El vídeo pasa al hueco principal y lo ocupa todo, sobre la ventana a pantalla completa. */
  protected enterVideoFullscreen(): Promise<void> {
    return this.run(async () => {
      this.slot.set('main');
      if (await this.engine.setWindowFullscreen(true)) {
        this.fullscreen.set('video');
      }
    });
  }

  protected exitFullscreen(): Promise<void> {
    if (this.fullscreen() === 'off') {
      return Promise.resolve();
    }
    return this.run(async () => {
      await this.engine.setWindowFullscreen(false);
      this.fullscreen.set('off');
    });
  }

  protected toggleEngineFullscreen(): Promise<void> {
    const enable = !this.engineFullscreen();
    return this.run(async () => {
      await this.engine.setEngineFullscreen(enable);
      this.engineFullscreen.set(enable);
    });
  }

  /** Comprueba qué ventana recibe la entrada en el centro del vídeo y de cada control superpuesto. */
  protected readWindows(): Promise<void> {
    const ratio = window.devicePixelRatio || 1;
    const points: ClientPoint[] = Array.from(
      this.host.querySelectorAll<HTMLElement>('.stage, .stage [data-surface-hole] button, .stage input'),
    ).map((element) => {
      const box = toPhysicalRect(element.getBoundingClientRect(), ratio);
      return { x: box.x + Math.floor(box.width / 2), y: box.y + Math.floor(box.height / 2) };
    });
    return this.run(async () => this.report.set(await this.engine.windowReport(points)));
  }

  protected chooseKind(event: Event): void {
    this.kind.set((event.target as HTMLSelectElement).value as EngineKind);
  }

  protected text(event: Event): string {
    return (event.target as HTMLInputElement).value;
  }

  protected checked(event: Event): boolean {
    return (event.target as HTMLInputElement).checked;
  }

  protected seconds(value: number | null, digits = 1): string {
    return value === null ? '—' : `${value.toFixed(digits)} s`;
  }

  protected rect(rect: SurfaceRect): string {
    return `${rect.x}, ${rect.y} · ${rect.width}×${rect.height}`;
  }

  protected choose(item: string): void {
    this.menuChoice.set(item);
    this.menuOpen.set(false);
  }

  private async run(action: () => Promise<void>): Promise<void> {
    if (this.busy()) {
      return;
    }
    this.busy.set(true);
    this.error.set(null);
    try {
      await action();
    } catch (error) {
      this.error.set(describeError(error));
    } finally {
      this.busy.set(false);
    }
  }
}
