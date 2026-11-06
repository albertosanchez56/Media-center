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
  TrackReport,
  WindowReport,
} from '../../engine/engine.service';
import { VideoSurfaceDirective, toPhysicalRect } from '../../engine/video-surface.directive';
import { describeError } from '../../shared/describe-error';

export type StageSize = 'small' | 'medium' | 'expanded';

export const ENGINE_OPTIONS: readonly { kind: EngineKind; label: string }[] = [
  { kind: 'libmpv', label: 'A · libmpv dentro de la aplicación' },
  { kind: 'mpv-process', label: 'B · proceso mpv.exe' },
];

export const STAGE_SIZES: readonly { size: StageSize; label: string }[] = [
  { size: 'small', label: 'Pequeño' },
  { size: 'medium', label: 'Mediano' },
  { size: 'expanded', label: 'Ampliado' },
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
})
export class EngineDiagnosticsComponent {
  private readonly engine = inject(EngineService);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;

  protected readonly engineOptions = ENGINE_OPTIONS;
  protected readonly stageSizes = STAGE_SIZES;
  protected readonly menuItems = MENU_ITEMS;

  protected readonly state = this.engine.state;
  protected readonly surfaceError = this.engine.surfaceError;
  protected readonly active = computed(() => this.state().kind !== null);

  protected readonly kind = signal<EngineKind>('libmpv');
  protected readonly path = signal('');
  protected readonly associatedVideo = signal('');
  protected readonly d3d11Flip = signal(false);

  protected readonly size = signal<StageSize>('medium');
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
      if (this.active()) {
        this.engine.stop().catch(() => undefined);
      }
    });
  }

  protected readonly mainPath = computed(() => cleanPath(this.path()));
  protected readonly associatedPath = computed(() => cleanPath(this.associatedVideo()));

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
