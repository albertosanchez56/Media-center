import { TestBed } from '@angular/core/testing';
import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { EngineService, EngineState, IDLE_ENGINE_STATE } from './engine.service';

const PLAYING: EngineState = {
  ...IDLE_ENGINE_STATE,
  kind: 'libmpv',
  path: 'C:\\pruebas\\cancion.mp3',
  timePos: 12.5,
};

describe('EngineService', () => {
  let service: EngineService;

  beforeEach(() => {
    service = TestBed.inject(EngineService);
  });

  afterEach(() => clearMocks());

  it('should send the open request and keep the returned state', async () => {
    mockIPC((cmd, args) => {
      expect(cmd).toBe('engine_open');
      expect(args).toEqual({
        request: {
          kind: 'libmpv',
          path: PLAYING.path,
          associatedVideo: null,
          d3d11Flip: false,
        },
      });
      return PLAYING;
    });

    await service.open({ kind: 'libmpv', path: PLAYING.path!, associatedVideo: null, d3d11Flip: false });
    expect(service.state()).toEqual(PLAYING);
  });

  it('should turn the Rust error text into an Error', async () => {
    mockIPC(() => {
      throw 'No existe el archivo: C:\\falta.mp3';
    });

    await expect(service.seek(10)).rejects.toThrow('No existe el archivo: C:\\falta.mp3');
  });

  it('should follow the state emitted by Rust', async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });

    await service.connect();
    await emit('engine-state', PLAYING);
    expect(service.state()).toEqual(PLAYING);
  });

  it('should go back to the idle state after stopping', async () => {
    const commands: string[] = [];
    mockIPC((cmd) => {
      commands.push(cmd);
      return cmd === 'engine_open' ? PLAYING : undefined;
    });

    await service.open({ kind: 'mpv-process', path: 'a.mp3', associatedVideo: null, d3d11Flip: false });
    await service.stop();
    expect(commands).toEqual(['engine_open', 'engine_stop']);
    expect(service.state()).toEqual(IDLE_ENGINE_STATE);
  });

  it('should record surface failures instead of rejecting', async () => {
    mockIPC(() => {
      throw 'Rectángulo de superficie no válido';
    });
    const rect = { x: 0, y: 0, width: 0, height: 10 };
    await service.setSurface({ bounds: rect, visible: rect });
    expect(service.surfaceError()).toBe('Rectángulo de superficie no válido');

    mockIPC((cmd, args) => {
      expect(cmd).toBe('engine_set_holes');
      expect(args).toEqual({ holes: [] });
    });
    await service.setHoles([]);
    expect(service.surfaceError()).toBeNull();
  });

  it('should send a null placement to hide the surface', async () => {
    mockIPC((cmd, args) => {
      expect(cmd).toBe('engine_set_surface');
      expect(args).toEqual({ placement: null });
    });
    await service.setSurface(null);
    expect(service.surfaceError()).toBeNull();
  });

  it('should pass on the wheel turns forwarded by the video surface', async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const turns: unknown[] = [];

    await service.onSurfaceWheel((wheel) => turns.push(wheel));
    await emit('engine-surface-wheel', { delta: -120, horizontal: false });
    expect(turns).toEqual([{ delta: -120, horizontal: false }]);
  });

  it('should send track, subtitle and fullscreen requests', async () => {
    const calls: [string, unknown][] = [];
    mockIPC((cmd, args) => {
      calls.push([cmd, args]);
      return cmd === 'engine_set_window_fullscreen' ? true : undefined;
    });

    await service.selectTrack('audio', 2);
    await service.selectTrack('subtitle', null);
    await service.addSubtitle('C:\\pruebas\\externo.srt');
    expect(await service.setWindowFullscreen(true)).toBe(true);
    await service.setEngineFullscreen(false);

    expect(calls).toEqual([
      ['engine_select_track', { kind: 'audio', id: 2 }],
      ['engine_select_track', { kind: 'subtitle', id: null }],
      ['engine_add_subtitle', { path: 'C:\\pruebas\\externo.srt' }],
      ['engine_set_window_fullscreen', { enabled: true }],
      ['engine_set_engine_fullscreen', { enabled: false }],
    ]);
  });

  it('should reject connect when Tauri is not available', async () => {
    await expect(service.connect()).rejects.toThrow();
  });
});
