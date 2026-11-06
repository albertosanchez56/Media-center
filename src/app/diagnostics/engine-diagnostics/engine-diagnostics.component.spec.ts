import { signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import {
  EngineService,
  EngineState,
  IDLE_ENGINE_STATE,
  OpenRequest,
} from '../../engine/engine.service';
import { EngineDiagnosticsComponent } from './engine-diagnostics.component';

class EngineStub {
  readonly state = signal<EngineState>(IDLE_ENGINE_STATE);
  readonly surfaceError = signal<string | null>(null);
  readonly opened: OpenRequest[] = [];
  readonly calls: string[] = [];
  openError: string | null = null;

  connect = async () => undefined;
  setSurface = async () => undefined;
  setHoles = async () => undefined;
  onSurfaceWheel = async () => () => undefined;

  open = async (request: OpenRequest) => {
    this.opened.push(request);
    if (this.openError) {
      throw new Error(this.openError);
    }
    this.state.set({ ...IDLE_ENGINE_STATE, kind: request.kind, path: request.path });
  };

  setPause = async (paused: boolean) => void this.calls.push(`pause:${paused}`);
  seek = async (seconds: number) => void this.calls.push(`seek:${seconds}`);
  stop = async () => {
    this.calls.push('stop');
    this.state.set(IDLE_ENGINE_STATE);
  };
}

describe('EngineDiagnosticsComponent', () => {
  let engine: EngineStub;
  let fixture: ComponentFixture<EngineDiagnosticsComponent>;
  let element: HTMLElement;

  beforeEach(async () => {
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(): void {}
        disconnect(): void {}
      },
    );
    engine = new EngineStub();
    await TestBed.configureTestingModule({
      imports: [EngineDiagnosticsComponent],
      providers: [{ provide: EngineService, useValue: engine }],
    }).compileComponents();

    fixture = TestBed.createComponent(EngineDiagnosticsComponent);
    element = fixture.nativeElement as HTMLElement;
    await fixture.whenStable();
  });

  afterEach(() => vi.unstubAllGlobals());

  function button(label: string): HTMLButtonElement {
    const found = Array.from(element.querySelectorAll('button')).find(
      (candidate) => candidate.textContent?.trim() === label,
    );
    if (!found) {
      throw new Error(`No se encuentra el botón ${label}`);
    }
    return found;
  }

  function typeInto(index: number, value: string): void {
    const input = element.querySelectorAll<HTMLInputElement>('.form input[type="text"]')[index];
    input.value = value;
    input.dispatchEvent(new Event('input'));
  }

  const typePath = (value: string) => typeInto(0, value);
  const typeAssociated = (value: string) => typeInto(1, value);

  it('should keep the playback controls disabled until something is open', () => {
    expect(button('Abrir').disabled).toBe(true);
    for (const label of ['Pausa', '−10 s', '+10 s', 'Detener']) {
      expect(button(label).disabled).toBe(true);
    }
  });

  it('should open the chosen integration with the trimmed paths', async () => {
    const select = element.querySelector('select')!;
    select.value = 'mpv-process';
    select.dispatchEvent(new Event('change'));
    typePath('  C:\\pruebas\\cancion.mp3 ');
    await fixture.whenStable();

    button('Abrir').click();
    await fixture.whenStable();

    expect(engine.opened).toEqual([
      { kind: 'mpv-process', path: 'C:\\pruebas\\cancion.mp3', associatedVideo: null, d3d11Flip: false },
    ]);
    expect(button('Pausa').disabled).toBe(false);
  });

  it('should open a video with its own sound without an associated video', async () => {
    typePath('C:\\pruebas\\pelicula.mp4');
    await fixture.whenStable();

    button('Abrir').click();
    await fixture.whenStable();

    expect(engine.opened[0].path).toBe('C:\\pruebas\\pelicula.mp4');
    expect(engine.opened[0].associatedVideo).toBeNull();
  });

  it('should remove the quotes added by "Copy as path"', async () => {
    typePath('"C:\\Mis vídeos\\pelicula.mp4"');
    typeAssociated('  "C:\\Mis vídeos\\clip.mp4" ');
    await fixture.whenStable();

    button('Abrir').click();
    await fixture.whenStable();

    expect(engine.opened[0].path).toBe('C:\\Mis vídeos\\pelicula.mp4');
    expect(engine.opened[0].associatedVideo).toBe('C:\\Mis vídeos\\clip.mp4');
  });

  it('should explain the missing main file when only the associated video is filled in', async () => {
    expect(element.querySelector('.missing-main')).toBeNull();
    typeAssociated('C:\\pruebas\\pelicula.mp4');
    await fixture.whenStable();

    expect(button('Abrir').disabled).toBe(true);
    expect(element.querySelector('.missing-main')?.textContent).toContain('Falta el archivo principal');
  });

  it('should send pause, seek and stop to the engine', async () => {
    engine.state.set({ ...IDLE_ENGINE_STATE, kind: 'libmpv' });
    await fixture.whenStable();

    button('Pausa').click();
    await fixture.whenStable();
    button('+10 s').click();
    await fixture.whenStable();
    button('Detener').click();
    await fixture.whenStable();

    expect(engine.calls).toEqual(['pause:true', 'seek:10', 'stop']);
  });

  it('should show the error returned when opening fails', async () => {
    engine.openError = 'No existe el archivo: C:\\falta.mp3';
    typePath('C:\\falta.mp3');
    await fixture.whenStable();

    button('Abrir').click();
    await fixture.whenStable();

    expect(element.querySelector('[role="alert"]')?.textContent).toContain('No existe el archivo');
  });

  it('should count the events received by the HTML controls over the video', async () => {
    const overlay = element.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')[1];
    overlay.checked = true;
    overlay.dispatchEvent(new Event('change'));
    await fixture.whenStable();

    button('Botón superpuesto').click();
    button('Menú').click();
    await fixture.whenStable();
    button('Segunda opción').click();
    element
      .querySelector('input[aria-label="Campo superpuesto"]')!
      .dispatchEvent(new KeyboardEvent('keydown', { key: 'a' }));
    await fixture.whenStable();

    expect(element.querySelector('.overlay-clicks')?.textContent).toBe('1');
    expect(element.querySelector('.menu-choice')?.textContent).toBe('Segunda opción');
    expect(element.querySelector('.key-count')?.textContent).toBe('1');
    expect(element.querySelector('[role="menu"]')).toBeNull();
  });

  it('should stop the playback when the view is closed', async () => {
    engine.state.set({ ...IDLE_ENGINE_STATE, kind: 'libmpv' });
    fixture.destroy();
    expect(engine.calls).toEqual(['stop']);
  });
});
