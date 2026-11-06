import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { EngineService, SurfacePlacement, SurfaceRect, SurfaceWheel } from './engine.service';
import {
  VideoSurfaceDirective,
  WHEEL_STEP_PX,
  intersectBoxes,
  toPhysicalRect,
} from './video-surface.directive';

function box(left: number, top: number, width: number, height: number): DOMRect {
  return { left, top, right: left + width, bottom: top + height, width, height } as DOMRect;
}

describe('toPhysicalRect', () => {
  it('should scale CSS pixels by the device pixel ratio', () => {
    expect(toPhysicalRect(box(10, 20, 100, 50), 1.5)).toEqual({ x: 15, y: 30, width: 150, height: 75 });
  });

  it('should keep adjacent boxes without gaps when rounding', () => {
    const first = toPhysicalRect(box(0, 0, 33.4, 10), 1.25);
    const second = toPhysicalRect(box(33.4, 0, 33.4, 10), 1.25);
    expect(first.x + first.width).toBe(second.x);
  });

  it('should be relative to the origin when one is given', () => {
    expect(toPhysicalRect(box(110, 70, 20, 10), 2, box(100, 50, 300, 200))).toEqual({
      x: 20,
      y: 40,
      width: 40,
      height: 20,
    });
  });
});

describe('intersectBoxes', () => {
  it('should keep the common part', () => {
    expect(intersectBoxes([box(0, 0, 100, 100), box(50, 60, 100, 100)])).toEqual({
      left: 50,
      top: 60,
      right: 100,
      bottom: 100,
    });
  });

  it('should return null when the boxes do not overlap', () => {
    expect(intersectBoxes([box(0, 0, 100, 100), box(0, 100, 100, 100)])).toBeNull();
  });
});

@Component({
  imports: [VideoSurfaceDirective],
  template: `
    <div class="scroller" style="overflow: auto">
      <div class="stage" appVideoSurface [clipHoles]="clip()">
        <button class="hole" data-surface-hole>Encima</button>
      </div>
    </div>
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
class HostComponent {
  readonly clip = signal(false);
}

describe('VideoSurfaceDirective', () => {
  let placements: (SurfacePlacement | null)[];
  let holes: SurfaceRect[][];
  let wheelHandler: ((wheel: SurfaceWheel) => void) | null;
  let stageBox: DOMRect;
  let scrollerBox: DOMRect;

  beforeEach(() => {
    placements = [];
    holes = [];
    wheelHandler = null;
    stageBox = box(100, 50, 640, 360);
    scrollerBox = box(0, 0, 1000, 700);
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(): void {}
        disconnect(): void {}
      },
    );
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) =>
      setTimeout(() => callback(0), 0),
    );
    vi.stubGlobal('devicePixelRatio', 1.5);
    vi.stubGlobal('innerWidth', 1200);
    vi.stubGlobal('innerHeight', 800);

    const engineStub: Pick<EngineService, 'setSurface' | 'setHoles' | 'onSurfaceWheel'> = {
      setSurface: async (placement) => void placements.push(placement),
      setHoles: async (rects) => void holes.push(rects),
      onSurfaceWheel: async (handler) => {
        wheelHandler = handler;
        return () => (wheelHandler = null);
      },
    };
    TestBed.configureTestingModule({
      providers: [{ provide: EngineService, useValue: engineStub }],
    });
  });

  afterEach(() => vi.unstubAllGlobals());

  async function render() {
    const fixture = TestBed.createComponent(HostComponent);
    const element = fixture.nativeElement as HTMLElement;
    const scroller = element.querySelector<HTMLElement>('.scroller')!;
    scroller.getBoundingClientRect = () => scrollerBox;
    Object.defineProperty(scroller, 'clientWidth', { get: () => scrollerBox.width });
    Object.defineProperty(scroller, 'clientHeight', { get: () => scrollerBox.height });
    element.querySelector<HTMLElement>('.stage')!.getBoundingClientRect = () => stageBox;
    element.querySelector<HTMLElement>('.hole')!.getBoundingClientRect = () => box(110, 60, 80, 20);
    await settle(fixture);
    return { fixture, scroller };
  }

  async function settle(fixture: { whenStable(): Promise<unknown> }): Promise<void> {
    await fixture.whenStable();
    await new Promise((resolve) => setTimeout(resolve, 5));
  }

  it('should place the surface over the host in physical pixels', async () => {
    await render();
    const whole = { x: 150, y: 75, width: 960, height: 540 };
    expect(placements).toEqual([{ bounds: whole, visible: whole }]);
    expect(holes).toEqual([[]]);
  });

  it('should only show the part left visible by the scroll container', async () => {
    stageBox = box(100, 500, 640, 360);
    await render();
    expect(placements).toEqual([
      {
        bounds: { x: 150, y: 750, width: 960, height: 540 },
        visible: { x: 150, y: 750, width: 960, height: 300 },
      },
    ]);
  });

  it('should hide the surface when it is scrolled out of view', async () => {
    stageBox = box(100, 800, 640, 360);
    await render();
    expect(placements).toEqual([null]);
  });

  it('should cut holes for the marked elements only when asked to', async () => {
    const { fixture } = await render();
    fixture.componentInstance.clip.set(true);
    await settle(fixture);
    expect(holes.at(-1)).toEqual([{ x: 15, y: 15, width: 120, height: 30 }]);
    expect(placements).toHaveLength(1);
  });

  it('should not send a zero size, as happens when the window is minimized', async () => {
    stageBox = box(0, 0, 0, 0);
    await render();
    expect(placements).toEqual([]);
  });

  it('should scroll the container once per wheel turn over the video', async () => {
    const { scroller } = await render();
    Object.defineProperty(scroller, 'scrollHeight', { get: () => 2000 });
    const scrolls: ScrollToOptions[] = [];
    scroller.scrollBy = ((options: ScrollToOptions) => scrolls.push(options)) as typeof scroller.scrollBy;

    wheelHandler?.({ delta: -120, horizontal: false });
    wheelHandler?.({ delta: 240, horizontal: false });
    expect(scrolls).toEqual([{ top: WHEEL_STEP_PX }, { top: -2 * WHEEL_STEP_PX }]);
  });

  it('should hide the surface and stop listening to the wheel when the host is destroyed', async () => {
    const { fixture } = await render();
    expect(wheelHandler).not.toBeNull();
    fixture.destroy();
    expect(placements.at(-1)).toBeNull();
    expect(wheelHandler).toBeNull();
  });
});
