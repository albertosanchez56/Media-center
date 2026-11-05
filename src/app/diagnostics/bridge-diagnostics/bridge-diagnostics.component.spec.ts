import { TestBed } from '@angular/core/testing';
import { BridgeResult, BridgeService } from '../../bridge/bridge.service';
import { BridgeDiagnosticsComponent } from './bridge-diagnostics.component';

describe('BridgeDiagnosticsComponent', () => {
  let pending: ((result: BridgeResult) => void) | null;
  let calls: boolean[];

  beforeEach(async () => {
    pending = null;
    calls = [];
    const bridgeStub: Pick<BridgeService, 'check'> = {
      check: (fail) => {
        calls.push(fail);
        return new Promise((resolve) => (pending = resolve));
      },
    };

    await TestBed.configureTestingModule({
      imports: [BridgeDiagnosticsComponent],
      providers: [{ provide: BridgeService, useValue: bridgeStub }],
    }).compileComponents();
  });

  async function clickAndResolve(index: number, result: BridgeResult): Promise<HTMLElement> {
    const fixture = TestBed.createComponent(BridgeDiagnosticsComponent);
    await fixture.whenStable();
    const compiled = fixture.nativeElement as HTMLElement;
    compiled.querySelectorAll('button')[index].click();
    fixture.detectChanges();
    pending?.(result);
    await fixture.whenStable();
    fixture.detectChanges();
    return compiled;
  }

  it('should show the Rust response', async () => {
    const compiled = await clickAndResolve(0, { kind: 'success', message: 'Respuesta de Rust' });
    expect(calls).toEqual([false]);
    expect(compiled.querySelector('.success')?.textContent).toContain('Respuesta de Rust');
  });

  it('should show an intentional failure differently from a communication error', async () => {
    const compiled = await clickAndResolve(1, { kind: 'intentional-error', message: 'Fallo' });
    expect(calls).toEqual([true]);
    expect(compiled.querySelector('.intentional-error')?.textContent).toContain('Fallo provocado');
    expect(compiled.querySelector('.communication-error')).toBeNull();
  });

  it('should show a communication error', async () => {
    const compiled = await clickAndResolve(0, { kind: 'communication-error', message: 'Sin Tauri' });
    expect(compiled.querySelector('.communication-error')?.textContent).toContain('Sin Tauri');
    expect(compiled.querySelector('.intentional-error')).toBeNull();
  });

  it('should disable both buttons while a check is running', async () => {
    const fixture = TestBed.createComponent(BridgeDiagnosticsComponent);
    await fixture.whenStable();
    const compiled = fixture.nativeElement as HTMLElement;
    const buttons = Array.from(compiled.querySelectorAll('button'));

    buttons[0].click();
    fixture.detectChanges();
    expect(buttons.every((button) => button.disabled)).toBe(true);
    buttons[1].click();
    expect(calls).toEqual([false]);

    pending?.({ kind: 'success', message: 'ok' });
    await fixture.whenStable();
    fixture.detectChanges();
    expect(buttons.every((button) => button.disabled)).toBe(false);
  });
});
