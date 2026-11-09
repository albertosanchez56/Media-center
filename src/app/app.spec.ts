import { ComponentFixture, TestBed } from '@angular/core/testing';
import { App } from './app';
import { BridgeService } from './bridge/bridge.service';
import { FileDialog } from './import/file-dialog';
import { ImportTransport } from './import/import-transport';

describe('App', () => {
  let fixture: ComponentFixture<App>;
  let element: HTMLElement;

  afterEach(() => vi.unstubAllGlobals());

  beforeEach(async () => {
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(): void {}
        disconnect(): void {}
      },
    );
    const bridgeStub: Pick<BridgeService, 'check'> = {
      check: () => new Promise(() => undefined),
    };

    await TestBed.configureTestingModule({
      imports: [App],
      providers: [
        { provide: BridgeService, useValue: bridgeStub },
        {
          provide: FileDialog,
          useValue: { openFiles: () => Promise.resolve(null), openFolder: () => Promise.resolve(null) },
        },
        {
          provide: ImportTransport,
          useValue: {
            folderStatus: () => Promise.resolve({ path: 'C:\\Biblioteca', state: 'proposed' }),
            start: () => Promise.resolve(),
            cancel: () => Promise.resolve(),
            listen: () => Promise.resolve(() => undefined),
          },
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    element = fixture.nativeElement as HTMLElement;
  });

  it('should create the app', () => {
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render the Media Center brand', () => {
    expect(element.querySelector('.brand')?.textContent).toContain('Media Center');
  });

  it('should render the four zones', () => {
    expect(element.querySelector('nav[aria-label="Bibliotecas"]')).not.toBeNull();
    expect(element.querySelector('main')).not.toBeNull();
    expect(element.querySelector('aside[aria-label="Ahora suena"]')).not.toBeNull();
    expect(element.querySelector('section[aria-label="Controles de reproducción"]')).not.toBeNull();
  });

  it('should show the music library by default', () => {
    expect(element.querySelector('main h1')?.textContent).toContain('Música');
    expect(element.querySelector('main .import-button')?.textContent).toContain('Importar');
  });

  it('should change the central title when another library is chosen', async () => {
    const buttons = element.querySelectorAll<HTMLButtonElement>('nav[aria-label="Bibliotecas"] button');
    buttons[2].click();
    await fixture.whenStable();
    expect(element.querySelector('main h1')?.textContent).toContain('Películas');
  });

  it('should open the bridge check from the diagnostics entry', async () => {
    element.querySelector<HTMLButtonElement>('.diagnostics')!.click();
    await fixture.whenStable();
    expect(element.querySelector('main h1')?.textContent).toContain('Diagnóstico');
    expect(element.querySelector('main .import-button')).toBeNull();
    expect(element.querySelector('main')?.textContent).toContain('Probar conexión con Rust');
    expect(element.querySelector('main section[aria-label="Prueba del motor multimedia"]')).not.toBeNull();
  });
});
