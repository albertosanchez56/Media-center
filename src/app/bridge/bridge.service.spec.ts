import { TestBed } from '@angular/core/testing';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { BridgeService } from './bridge.service';

describe('BridgeService', () => {
  let service: BridgeService;

  beforeEach(() => {
    service = TestBed.inject(BridgeService);
  });

  afterEach(() => clearMocks());

  it('should return the response of bridge_check', async () => {
    mockIPC((cmd, args) => {
      expect(cmd).toBe('bridge_check');
      expect(args).toEqual({ fail: false });
      return 'Respuesta de Rust: puente operativo';
    });

    expect(await service.check(false)).toEqual({
      kind: 'success',
      message: 'Respuesta de Rust: puente operativo',
    });
  });

  it('should classify the intentional failure returned by Rust', async () => {
    mockIPC(() => {
      throw 'INTENTIONAL_FAILURE: Fallo provocado a propósito en Rust';
    });

    expect(await service.check(true)).toEqual({
      kind: 'intentional-error',
      message: 'Fallo provocado a propósito en Rust',
    });
  });

  it('should classify any other rejection as a communication error', async () => {
    mockIPC(() => {
      throw 'Command bridge_check not found';
    });

    expect(await service.check(true)).toEqual({
      kind: 'communication-error',
      message: 'Command bridge_check not found',
    });
  });

  it('should report a communication error when Tauri is not available', async () => {
    const result = await service.check(false);
    expect(result.kind).toBe('communication-error');
  });
});
