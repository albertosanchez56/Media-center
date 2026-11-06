import {
  DestroyRef,
  Directive,
  ElementRef,
  afterNextRender,
  effect,
  inject,
  input,
} from '@angular/core';
import { EngineService, SurfaceRect, SurfaceWheel } from './engine.service';

/** Marca los elementos HTML que deben verse sobre el vídeo cuando se recortan huecos. */
export const SURFACE_HOLE_ATTRIBUTE = 'data-surface-hole';

/** Píxeles CSS por paso de rueda (120 unidades), como desplaza WebView2 en Windows. */
export const WHEEL_STEP_PX = 100;
const WHEEL_DELTA = 120;

interface CssBox {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** Convierte una caja CSS en píxeles físicos, relativa a `origin` (por defecto, el área cliente). */
export function toPhysicalRect(box: CssBox, ratio: number, origin: CssBox | null = null): SurfaceRect {
  const left = Math.round((box.left - (origin?.left ?? 0)) * ratio);
  const top = Math.round((box.top - (origin?.top ?? 0)) * ratio);
  return {
    x: left,
    y: top,
    width: Math.round((box.right - (origin?.left ?? 0)) * ratio) - left,
    height: Math.round((box.bottom - (origin?.top ?? 0)) * ratio) - top,
  };
}

/** Intersección de cajas CSS; `null` si no se solapan. */
export function intersectBoxes(boxes: CssBox[]): CssBox | null {
  const box = boxes.reduce((acc, next) => ({
    left: Math.max(acc.left, next.left),
    top: Math.max(acc.top, next.top),
    right: Math.min(acc.right, next.right),
    bottom: Math.min(acc.bottom, next.bottom),
  }));
  return box.right > box.left && box.bottom > box.top ? box : null;
}

function clips(element: Element): boolean {
  const style = getComputedStyle(element);
  return [style.overflow, style.overflowX, style.overflowY].some(
    (value) => value !== '' && value !== 'visible',
  );
}

/** Zona donde se ve el contenido: sin bordes ni barras de desplazamiento. */
function clientBox(element: HTMLElement): CssBox {
  const rect = element.getBoundingClientRect();
  const left = rect.left + element.clientLeft;
  const top = rect.top + element.clientTop;
  return { left, top, right: left + element.clientWidth, bottom: top + element.clientHeight };
}

/** Antepasados que recortan su contenido. Un elemento `fixed` solo lo recorta la ventana. */
function clippingAncestors(host: HTMLElement): HTMLElement[] {
  if (getComputedStyle(host).position === 'fixed') {
    return [];
  }
  const found: HTMLElement[] = [];
  for (let element = host.parentElement; element && element !== document.body; element = element.parentElement) {
    if (clips(element)) {
      found.push(element);
    }
  }
  return found;
}

/**
 * Coloca la superficie nativa de vídeo sobre el elemento anfitrión y la sigue al cambiar de tamaño
 * o de posición. La ventana nativa no la recorta el `overflow` del HTML, así que se le envía la
 * parte visible. Con `clipHoles` recorta en ella los elementos marcados con `data-surface-hole`.
 */
@Directive({ selector: '[appVideoSurface]' })
export class VideoSurfaceDirective {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
  private readonly engine = inject(EngineService);

  readonly clipHoles = input(false);

  private frame: number | null = null;
  private lastPlacement = '';
  private lastHoles = '';

  constructor() {
    const schedule = () => this.schedule();
    const resize = new ResizeObserver(schedule);
    const mutations = new MutationObserver(schedule);
    let stopWheel: (() => void) | null = null;
    let destroyed = false;

    effect(() => {
      this.clipHoles();
      this.schedule();
    });

    afterNextRender(() => {
      resize.observe(this.host);
      mutations.observe(this.host, { childList: true, subtree: true, attributes: true });
      window.addEventListener('resize', schedule);
      document.addEventListener('scroll', schedule, true);
      this.engine
        .onSurfaceWheel((wheel) => this.scrollBy(wheel))
        .then((stop) => (destroyed ? stop() : (stopWheel = stop)))
        .catch(() => undefined);
      this.schedule();
    });

    inject(DestroyRef).onDestroy(() => {
      destroyed = true;
      stopWheel?.();
      resize.disconnect();
      mutations.disconnect();
      window.removeEventListener('resize', schedule);
      document.removeEventListener('scroll', schedule, true);
      if (this.frame !== null) {
        cancelAnimationFrame(this.frame);
      }
      void this.engine.setSurface(null);
    });
  }

  /** Vuelve a medir en el siguiente fotograma; agrupa los cambios que llegan seguidos. */
  schedule(): void {
    this.frame ??= requestAnimationFrame(() => {
      this.frame = null;
      this.sync();
    });
  }

  /** Desplaza el primer contenedor que puede moverse en esa dirección, como haría la rueda. */
  scrollBy(wheel: SurfaceWheel): void {
    const pixels = (wheel.delta / WHEEL_DELTA) * WHEEL_STEP_PX;
    const target = clippingAncestors(this.host).find((element) =>
      wheel.horizontal
        ? element.scrollWidth > element.clientWidth
        : element.scrollHeight > element.clientHeight,
    );
    if (wheel.horizontal) {
      target?.scrollBy({ left: pixels });
    } else {
      target?.scrollBy({ top: -pixels });
    }
  }

  private sync(): void {
    const ratio = window.devicePixelRatio || 1;
    const box = this.host.getBoundingClientRect();
    const bounds = toPhysicalRect(box, ratio);
    // Al minimizar, el área cliente mide cero; la superficie se queda donde estaba.
    if (bounds.width < 1 || bounds.height < 1) {
      return;
    }

    const viewport = { left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight };
    const visibleBox = intersectBoxes([
      box,
      viewport,
      ...clippingAncestors(this.host).map(clientBox),
    ]);
    const placement = visibleBox ? { bounds, visible: toPhysicalRect(visibleBox, ratio) } : null;

    const holes = this.clipHoles()
      ? Array.from(this.host.querySelectorAll<HTMLElement>(`[${SURFACE_HOLE_ATTRIBUTE}]`))
          .map((element) => toPhysicalRect(element.getBoundingClientRect(), ratio, box))
          .filter((hole) => hole.width > 0 && hole.height > 0)
      : [];

    const holesKey = JSON.stringify(holes);
    if (holesKey !== this.lastHoles) {
      this.lastHoles = holesKey;
      void this.engine.setHoles(holes);
    }
    const placementKey = JSON.stringify(placement);
    if (placementKey !== this.lastPlacement) {
      this.lastPlacement = placementKey;
      void this.engine.setSurface(placement);
    }
  }
}
